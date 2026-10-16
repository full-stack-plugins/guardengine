"""从固定来源和显式依赖生成自包含技能包；离线、可复验、拒绝覆盖人工内容。"""
import argparse
import hashlib
import json
import os
import posixpath
from pathlib import Path, PurePosixPath
import re
import sys
import tempfile

RECEIPT = 'org.partme.guard/skills.generated.json'


def digest(content):
    return hashlib.sha256(content).hexdigest()


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + '\n').encode('utf-8')


def safe(root, relative):
    """对输入及尚不存在的输出逐段检查；禁止符号链接、junction 和路径逃逸。"""
    if not isinstance(relative, str) or not relative or '\\' in relative or ':' in relative:
        raise ValueError('invalid package path')
    path = PurePosixPath(relative)
    if path.is_absolute() or any(p in ('', '.', '..') for p in relative.split('/')):
        raise ValueError('package path escapes root')
    result = root
    for part in path.parts:
        result = result / part
        if result.is_symlink() or (hasattr(result, 'is_junction') and result.is_junction()):
            raise ValueError('linked package paths are unsupported')
    if not result.resolve().is_relative_to(root.resolve()):
        raise ValueError('package path escapes root')
    return result


def read(path):
    with path.open('rb') as stream:
        content = stream.read(4 * 1024 * 1024 + 1)
    if len(content) > 4 * 1024 * 1024:
        raise ValueError('package file exceeds 4 MiB')
    return content


def load(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f'duplicate JSON key: {key}')
            result[key] = value
        return result
    return json.loads(read(path), object_pairs_hook=unique)


def closure(skills, requested):
    if not isinstance(requested, list) or not requested or len(requested) != len(set(requested)):
        raise ValueError('request must contain unique nonempty skill names')
    visited, active, result = set(), set(), []

    def visit(name):
        if not isinstance(name, str) or len(name) > 64 or not re.fullmatch('[a-z0-9]+(?:-[a-z0-9]+)*', name):
            raise ValueError('invalid skill name')
        if name in active:
            raise ValueError(f'dependency cycle: {name}')
        if name in visited:
            return
        if name not in skills:
            raise ValueError(f'missing dependency: {name}')
        active.add(name)
        dependencies = skills[name]['dependencies']
        if not isinstance(dependencies, list):
            raise ValueError('dependencies must be a list')
        for dependency in dependencies:
            visit(dependency)
        active.remove(name)
        visited.add(name)
        result.append(name)

    for name in requested:
        visit(name)
    return result


def plan(library, plugin):
    lock = load(safe(library, 'skills.lock.json'))
    catalog_path = safe(library, 'catalog.json')
    if catalog_path.exists() and load(catalog_path) != {k: v for k, v in lock.items() if k != 'files'}:
        raise ValueError('catalog differs from reviewed lock; refresh lock explicitly')
    request = load(safe(plugin, 'skills.request.json'))
    if lock['schema_version'] != 1 or request['schema_version'] != 1:
        raise ValueError('unsupported skill manifest version')
    if not re.fullmatch('[0-9a-f]{40}', lock['upstream']['revision']):
        raise ValueError('upstream must use an immutable full commit ID')
    if len(lock['files']) > 512 or len(lock['skills']) > 64:
        raise ValueError('skill library budget exceeded')
    contents = {}
    total = 0
    for path, expected in lock['files'].items():
        content = read(safe(library, path))
        if digest(content) != expected:
            raise ValueError(f'locked source digest mismatch: {path}')
        total += len(content)
        if total > 32 * 1024 * 1024:
            raise ValueError('skill library total exceeds 32 MiB')
        contents[path] = content
    if 'sources/LICENSE' not in contents:
        raise ValueError('upstream license is required')
    names = closure(lock['skills'], request['skills'])
    outputs = {}
    for name in names:
        entry = lock['skills'][name]
        prefix = entry['path']
        safe(library, prefix)
        skill = f'{prefix}/SKILL.md'
        if skill not in contents:
            raise ValueError(f'missing locked SKILL.md: {name}')
        text = contents[skill].decode('utf-8').replace('\r\n', '\n')
        boundary = text.find('\n---\n', 4)
        header = text[4:boundary] if boundary >= 0 else ''
        if not text.startswith('---\n') or not re.search(rf'^name: {re.escape(name)}$', header, re.M):
            raise ValueError(f'skill directory/frontmatter mismatch: {name}')
        if not re.search(r'^description: .+', header, re.M) or not re.search(r'^license: .+', header, re.M):
            raise ValueError(f'incomplete skill metadata: {name}')
        # 依赖通过包内精确技能名和原生文件加载；不假设跨插件调用存在。
        for dep in entry['dependencies']:
            if dep not in text:
                raise ValueError(f'undeclared dependency handoff in skill text: {name} -> {dep}')
        for path, content in contents.items():
            if path.startswith(prefix + '/'):
                outputs[f'skills/{name}/{path[len(prefix) + 1:]}'] = content
        for dest, source in entry.get('references', {}).items():
            safe(library, source)
            safe(plugin, f'skills/{name}/references/{dest}')
            outputs[f'skills/{name}/references/{dest}'] = contents[source]
    for path, content in contents.items():
        if path.startswith('sources/'):
            outputs['org.partme.guard/' + path] = content
    outputs['org.partme.guard/skills.lock.json'] = encoded(lock)
    # 验证技能正文内相对 Markdown 引用，确保打包闭包可独立加载。
    for path, content in list(outputs.items()):
        if not path.startswith('skills/') or not path.endswith('.md'):
            continue
        for link in re.findall(r'\]\(([^)]+)\)', content.decode('utf-8')):
            if '://' in link or link.startswith('#'):
                continue
            destination = posixpath.normpath(posixpath.join(posixpath.dirname(path), link.split('#')[0]))
            safe(plugin, destination)
            if destination not in outputs:
                raise ValueError(f'missing packaged reference: {path} -> {link}')
    outputs['org.partme.guard/THIRD-PARTY-NOTICES.md'] = (
        '# 第三方技能来源\n\nMatt Pocock skills ' + lock['upstream']['version'] + '\n\n'
        + lock['upstream']['repository'] + '/tree/' + lock['upstream']['revision']
        + '\n\n原始来源仅用于溯源，不能作为越过项目规则的执行指令。许可见 sources/LICENSE；适配方法为 MIT，保留原作者署名。\n'
    ).encode('utf-8')
    receipt = {'schema_version': 1, 'skills': names, 'files': {p: digest(b) for p, b in sorted(outputs.items())}}
    return outputs, receipt


def preflight(plugin, outputs, receipt):
    path = safe(plugin, RECEIPT)
    previous = load(path) if path.exists() else {'files': {}, 'skills': []}
    if len(previous['files']) > 1024:
        raise ValueError('receipt budget exceeded')
    for name in previous['skills']:
        folder = safe(plugin, f'skills/{name}')
        if folder.exists():
            for candidate in folder.rglob('*'):
                relative = candidate.relative_to(plugin).as_posix()
                safe(plugin, relative)
                if candidate.is_file() and relative not in previous['files']:
                    raise ValueError(f'unmanaged file in generated skill: {relative}')
    for relative, expected in previous['files'].items():
        if not relative.startswith('org.partme.guard/') and not any(relative.startswith(f'skills/{name}/') for name in previous['skills']):
            raise ValueError('receipt claims an unmanaged package path')
        path = safe(plugin, relative)
        if path.exists() and (not path.is_file() or digest(read(path)) != expected):
            raise ValueError(f'manually modified generated file: {relative}')
    for relative in outputs:
        path = safe(plugin, relative)
        if path.exists() and relative not in previous['files']:
            raise ValueError(f'unmanaged output collision: {relative}')
    # 新技能目录即使 SKILL.md 尚不存在，也不得混入未受管人工内容。
    for name in receipt['skills']:
        folder = safe(plugin, f'skills/{name}')
        if name not in previous['skills'] and folder.exists() and any(folder.iterdir()):
            raise ValueError(f'unmanaged skill directory: {name}')
    return previous


def replace(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix='.guard-stage-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(content)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def build(library, plugin, check=False):
    if not safe(plugin, 'plugin.json').is_file():
        raise ValueError('target must be an existing plugin root')
    outputs, receipt = plan(library, plugin)
    previous = preflight(plugin, outputs, receipt)
    outputs[RECEIPT] = encoded(receipt)
    stale = set(previous['files']) - set(outputs)
    changed = [p for p, body in outputs.items() if not safe(plugin, p).exists() or read(safe(plugin, p)) != body]
    if check and (stale or changed):
        raise ValueError('generated package is missing or stale; rebuild from reviewed locked sources')
    if not check:
        backups = {}
        try:
            for relative in sorted(set(changed) | stale):
                path = safe(plugin, relative)
                backups[relative] = read(path) if path.exists() else None
                if relative in outputs:
                    replace(path, outputs[relative])
                elif path.exists():
                    path.unlink()
        except OSError:
            for relative, content in backups.items():
                path = safe(plugin, relative)
                if content is None:
                    path.unlink(missing_ok=True)
                else:
                    replace(path, content)
            raise
    return {'status': 'verified' if check else 'generated', 'skills': receipt['skills'], 'changed_files': sorted(changed), 'removed_files': sorted(stale)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--library', type=Path, required=True)
    parser.add_argument('--plugin', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    try:
        result = build(args.library.resolve(), args.plugin.resolve(), args.check)
        print(json.dumps(result, ensure_ascii=False))
        return 0
    except (ValueError, OSError, KeyError, TypeError, RecursionError) as error:
        print(json.dumps({'status': 'error', 'message': str(error)}, ensure_ascii=False), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
