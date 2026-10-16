"""Validate cross-repository task ownership and native ledger references.

This is an inventory gate, not a semantic acceptance or evidence authenticity gate.
Explicit verified declarations remain the responsibility of the evidence reviewer.
"""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys

TASK_ID = re.compile(r'\bEG-[A-Z]+\d{2,}\b')
REQ_ID = re.compile(r'\bEG-[A-Z]+-\d{3,}\b')
CHECKBOX = re.compile(r'^\s*-\s+\[([ xX])\]\s+(.*)$')
MAX_BYTES = 8 * 1024 * 1024


def markdown_source(source):
    """Only native Markdown headings count; fenced examples cannot define obligations."""
    visible = []
    fence = None
    for line in source.splitlines():
        marker = re.match(r'^ {0,3}(`{3,}|~{3,})(.*)$', line)
        if fence is not None:
            if marker and marker[1][0] == fence[0] and len(marker[1]) >= len(fence) and not marker[2].strip():
                fence = None
            continue
        if marker:
            fence = marker[1]
            continue
        visible.append(line)
    require(fence is None, 'unclosed code fence in authoritative source')
    return '\n'.join(visible)


def specification_requirements(source):
    source = markdown_source(source)
    headings = list(re.finditer(r'^### Requirement:\s+(EG-[A-Z]+-\d{3,})\s+[^\n]+$', source, re.M))
    result = {}
    for index, heading in enumerate(headings):
        identity = heading[1]
        require(identity not in result, f'duplicate requirement: {identity}')
        block = source[heading.end():headings[index + 1].start() if index + 1 < len(headings) else len(source)]
        scenarios = list(re.finditer(r'^#### Scenario:\s+([^\n]+)$', block, re.M))
        require(scenarios, f'{identity}: no native scenarios')
        names = set()
        for position, scenario in enumerate(scenarios):
            name = scenario[1].strip()
            require(name not in names, f'{identity}: duplicate scenario: {name}')
            names.add(name)
            body = block[scenario.end():scenarios[position + 1].start() if position + 1 < len(scenarios) else len(block)]
            require(re.search(r'^- \*\*WHEN\*\*\s+\S', body, re.M)
                    and re.search(r'^- \*\*THEN\*\*\s+\S', body, re.M), f'{identity}/{name}: missing WHEN/THEN behavior')
        result[identity] = names
    require(result, 'empty authoritative requirement source')
    return result


def acceptance_cases(source):
    result = {}
    for line in markdown_source(source).splitlines():
        match = re.match(r'^\|\s*(EG-AT-\d{3,})\s*\|', line)
        if match is None:
            continue
        identity = match[1]
        require(identity not in result, f'duplicate acceptance case: {identity}')
        tasks = TASK_ID.findall(line)
        require(tasks and len(tasks) == len(set(tasks)), f'{identity}: missing/duplicate acceptance tasks')
        result[identity] = set(tasks)
    require(result, 'empty authoritative acceptance source')
    return result


def require(condition, message):
    if not condition:
        raise ValueError(message)


def fields(value, required, optional=()):
    require(isinstance(value, dict), 'expected object')
    require(set(required) <= set(value) <= set(required) | set(optional),
            f'invalid fields: {sorted(value)}; required {sorted(required)}')


def strings(value, label):
    require(isinstance(value, list) and all(isinstance(v, str) and v for v in value), f'{label}: expected strings')
    require(len(value) == len(set(value)), f'{label}: duplicate reference')
    return set(value)


def safe_path(root, relative):
    require(isinstance(relative, str) and relative and '\\' not in relative, 'path must use relative slash notation')
    parts = PurePosixPath(relative).parts
    require(not PurePosixPath(relative).is_absolute() and ':' not in relative
            and all(p not in ('', '.', '..') for p in relative.split('/')),
            f'unsafe path: {relative}')
    current = root
    for part in parts:
        current = current / part
        info = current.lstat()
        require(not stat.S_ISLNK(info.st_mode) and not (getattr(info, 'st_file_attributes', 0) & 0x400),
                f'link/reparse path: {current}')
    resolved = current.resolve(strict=True)
    require(resolved.is_relative_to(root), f'path escapes root: {relative}')
    return resolved


def read_text(path):
    require(path.is_file(), f'not a regular file: {path}')
    require(path.stat().st_size <= MAX_BYTES, f'file too large: {path}')
    with path.open('rb') as stream:
        raw = stream.read(MAX_BYTES + 1)
    require(len(raw) <= MAX_BYTES, f'file too large: {path}')
    return raw.decode('utf-8-sig')


def native_tasks(source):
    result = {}
    total = completed = 0
    for line in markdown_source(source).splitlines():
        match = CHECKBOX.match(line)
        if not match:
            continue
        total += 1
        done = match[1].lower() == 'x'
        completed += int(done)
        # IDs in dependencies are references, never task definitions.
        description = re.split(r'依赖\s*[:：]', match[2], maxsplit=1)[0]
        definition = re.match(r'\s*(?:\d+(?:\.\d+)*[.)]?\s+)?(?:\*\*)?(EG-[A-Z]+\d{2,})\b', description)
        if definition is None:
            continue
        identity = definition[1]
        require(identity not in result, f'duplicate native task: {identity}')
        dep = re.search(r'依赖\s*[:：](.*?)(?:规格\s*[:：]|$)', line)
        req = re.search(r'规格\s*[:：](.*)$', line)
        require(dep is not None and req is not None, f'{identity}: missing dependency/spec inventory')
        dependencies = TASK_ID.findall(dep[1])
        requirements = REQ_ID.findall(req[1])
        require(len(set(dependencies)) == len(dependencies), f'{identity}: duplicate native dependency')
        require(requirements and len(set(requirements)) == len(requirements), f'{identity}: invalid requirements')
        owner = re.search(r'\bowner=([a-z][a-z0-9-]*)', description)
        result[identity] = {'done': done, 'dependencies': set(dependencies),
                            'requirements': set(requirements), 'owner': owner[1] if owner else None}
    return result, total, completed


def discover_ledgers(repository):
    """Discover active OpenSpec tasks and Superpowers plans; archives stay historical."""
    found = set()
    for section in ('openspec/changes', 'docs/superpowers/plans'):
        directory = repository / section
        if not directory.exists():
            continue
        safe_path(repository, section)
        for current, directories, files in os.walk(directory, followlinks=False):
            directories[:] = [name for name in directories if name != 'archive']
            for name in directories:
                safe_path(repository, (Path(current) / name).relative_to(repository).as_posix())
            for name in files:
                if not (name == 'tasks.md' or section.endswith('plans') and name.endswith('.md')):
                    continue
                relative = (Path(current) / name).relative_to(repository).as_posix()
                path = safe_path(repository, relative)
                if any(CHECKBOX.match(line) for line in markdown_source(read_text(path)).splitlines()):
                    found.add(relative)
                require(len(found) <= 4096, 'too many active ledgers')
    return found


def validate(repositories_root, inventory, audit=False):
    root = Path(repositories_root)
    require(root.is_absolute(), 'repositories root must be absolute')
    root = safe_path(root.anchor and Path(root.anchor), root.as_posix()[len(Path(root.anchor).as_posix()):].lstrip('/'))
    fields(inventory, {'schema_version', 'products', 'ledgers', 'tasks', 'ownership', 'specifications', 'acceptance'})
    require(inventory['schema_version'] == 'guard.delivery-inventory/v2', 'unsupported inventory schema')
    for key in ('products', 'ledgers', 'tasks', 'specifications', 'acceptance'):
        require(isinstance(inventory[key], list), f'{key}: expected list')
    require(inventory['products'] and inventory['ledgers'], 'empty product/ledger inventory')
    owners = {}
    name_to_owner = {}
    aliases = set()
    repositories = {}
    physical_repositories = set()
    for product in inventory['products']:
        fields(product, {'id', 'repository', 'logical_names'})
        identity = product['id']
        require(isinstance(identity, str) and identity, 'invalid product ID')
        names = strings(product['logical_names'], 'logical_names') | {identity}
        require(not (names & aliases), f'duplicate owner/alias: {identity}')
        aliases |= names
        name_to_owner.update({name: identity for name in names})
        repo = product['repository']
        require(repo not in repositories, f'duplicate repository owner: {repo}')
        resolved = safe_path(root, repo)
        require(resolved.is_dir(), f'not a repository directory: {repo}')
        require(resolved not in physical_repositories, f'duplicate physical repository owner: {repo}')
        physical_repositories.add(resolved)
        owners[identity] = repo
        repositories[repo] = resolved

    def reference(ref):
        fields(ref, {'repository', 'path'})
        require(ref['repository'] in repositories, f'unknown repository: {ref["repository"]}')
        return safe_path(repositories[ref['repository']], ref['path'])

    native = {}
    ledgers = {}
    physical_ledgers = set()
    summaries = []
    ownership_source = markdown_source(read_text(reference(inventory['ownership'])))
    authoritative_owners = {}
    for identity, owner in re.findall(r'^\|\s*(EG-[A-Z]+\d{2,})\s*\|\s*`([^`]+)`\s*\|', ownership_source, re.M):
        require(identity not in authoritative_owners, f'duplicate authoritative owner: {identity}')
        require(owner in name_to_owner, f'unknown authoritative owner: {owner}')
        authoritative_owners[identity] = name_to_owner[owner]
    require(authoritative_owners, 'empty authoritative ownership inventory')
    for ledger in inventory['ledgers']:
        fields(ledger, {'repository', 'path'}, {'ledger_kind'})
        key = (ledger['repository'], ledger['path'])
        require(key not in ledgers, f'duplicate ledger: {key}')
        kind = ledger.get('ledger_kind', 'stable_ids')
        require(kind in ('stable_ids', 'native_tasks'), f'invalid ledger kind: {kind}')
        path = reference({'repository': key[0], 'path': key[1]})
        require(path not in physical_ledgers, f'duplicate physical ledger: {key}')
        physical_ledgers.add(path)
        tasks, total, completed = native_tasks(read_text(path))
        require(total > 0, f'empty task ledger: {key}')
        require(kind != 'native_tasks' or not tasks, f'stable IDs cannot be hidden in native_tasks: {key}')
        require(kind != 'stable_ids' or bool(tasks), f'no stable IDs: {key}')
        ledgers[key] = kind
        summaries.append({'repository': key[0], 'path': key[1], 'ledger_kind': kind,
                          'total': total, 'completed': completed})
        for identity, task in tasks.items():
            require(identity not in native, f'duplicate native task: {identity}')
            native[identity] = dict(task, ledger=key)

    for repository, path in repositories.items():
        missing = discover_ledgers(path) - {key[1] for key in ledgers if key[0] == repository}
        require(not missing, f'unlisted active ledgers in {repository}: {sorted(missing)}')

    tasks = {}
    for task in inventory['tasks']:
        fields(task, {'id', 'owner', 'ledger', 'requirements', 'dependencies', 'evidence', 'status'})
        identity = task['id']
        require(isinstance(identity, str) and TASK_ID.fullmatch(identity), f'invalid task ID: {identity}')
        require(identity not in tasks, f'duplicate task ID/owner: {identity}')
        require(task['owner'] in owners, f'{identity}: unknown owner')
        require(identity in native, f'{identity}: no native task')
        require(authoritative_owners.get(identity) == task['owner'], f'{identity}: authoritative owner differs or missing')
        if native[identity]['owner']:
            require(name_to_owner.get(native[identity]['owner']) == task['owner'], f'{identity}: native owner differs')
        fields(task['ledger'], {'repository', 'path'})
        key = (task['ledger']['repository'], task['ledger']['path'])
        require(key == native[identity]['ledger'], f'{identity}: wrong native ledger')
        require(strings(task['requirements'], identity) == native[identity]['requirements'], f'{identity}: requirement inventory differs')
        require(strings(task['dependencies'], identity) == native[identity]['dependencies'], f'{identity}: dependency inventory differs')
        require(task['status'] in ('pending', 'partial', 'verified'), f'{identity}: invalid status')
        require(isinstance(task['evidence'], list), f'{identity}: expected evidence list')
        evidence = []
        for item in task['evidence']:
            evidence.append(str(reference(item)))
            read_text(Path(evidence[-1]))
        require(len(set(evidence)) == len(evidence), f'{identity}: duplicate evidence')
        if task['status'] == 'verified':
            require(native[identity]['done'] and evidence, f'{identity}: verified lacks native completion/evidence')
        tasks[identity] = task
    require(set(tasks) == set(native), f'missing tasks: {sorted(set(native) - set(tasks))}')
    requirements = {}
    physical_specs = set()
    for source in inventory['specifications']:
        fields(source, {'repository', 'path', 'requirements'})
        path = reference({'repository': source['repository'], 'path': source['path']})
        require(path not in physical_specs, 'duplicate physical specification source')
        physical_specs.add(path)
        native_requirements = specification_requirements(read_text(path))
        require(isinstance(source['requirements'], list), 'requirements: expected list')
        declared = {}
        for item in source['requirements']:
            fields(item, {'id', 'scenarios'})
            require(isinstance(item['id'], str) and REQ_ID.fullmatch(item['id']), 'invalid requirement ID')
            require(item['id'] not in declared, f'duplicate declared requirement: {item["id"]}')
            declared[item['id']] = strings(item['scenarios'], item['id'])
        require(set(declared) == set(native_requirements), 'requirement inventory differs from native source')
        for identity, scenarios in declared.items():
            require(scenarios == native_requirements[identity], f'{identity}: scenario inventory differs')
            require(identity not in requirements, f'duplicate requirement across sources: {identity}')
            requirements[identity] = scenarios
    used = {identity for task in tasks.values() for identity in task['requirements']}
    require(not (used - set(requirements)), f'unknown requirements: {sorted(used - set(requirements))}')
    require(used == set(requirements), f'unmapped requirements: {sorted(set(requirements) - used)}')
    cases = {}
    physical_cases = set()
    for source in inventory['acceptance']:
        fields(source, {'repository', 'path', 'cases'})
        path = reference({'repository': source['repository'], 'path': source['path']})
        require(path not in physical_cases, 'duplicate physical acceptance source')
        physical_cases.add(path)
        native_cases = acceptance_cases(read_text(path))
        require(isinstance(source['cases'], list), 'cases: expected list')
        declared = {}
        for case in source['cases']:
            fields(case, {'id', 'tasks'})
            require(isinstance(case['id'], str) and re.fullmatch(r'EG-AT-\d{3,}', case['id']), 'invalid acceptance ID')
            require(case['id'] not in declared, f'duplicate declared acceptance: {case["id"]}')
            declared[case['id']] = strings(case['tasks'], case['id'])
        require(set(declared) == set(native_cases), 'acceptance case inventory differs')
        for identity, case_tasks in declared.items():
            require(case_tasks == native_cases[identity], f'{identity}: acceptance task inventory differs')
            require(not (case_tasks - set(tasks)), f'{identity}: unknown acceptance tasks')
            require(identity not in cases, f'duplicate acceptance across sources: {identity}')
            cases[identity] = case_tasks
    require(requirements and cases, 'empty requirement/acceptance registry')
    require(set(authoritative_owners) == set(tasks), f'ownership/task inventory mismatch: {sorted(set(authoritative_owners) ^ set(tasks))}')
    unresolved = [{'task_id': identity, 'dependency': dependency}
                  for identity, task in sorted(tasks.items())
                  for dependency in sorted(task['dependencies']) if dependency not in tasks]
    if unresolved and not audit:
        raise ValueError(f'unknown dependencies: {json.dumps(unresolved, ensure_ascii=False)}')
    visiting, visited = set(), set()

    def visit(identity):
        require(identity not in visiting, f'dependency cycle: {identity}')
        if identity in visited:
            return
        visiting.add(identity)
        for dependency in tasks[identity]['dependencies']:
            if dependency in tasks:
                visit(dependency)
        visiting.remove(identity)
        visited.add(identity)

    for identity in tasks:
        visit(identity)
    return {'schema_version': 'guard.delivery-inventory-result/v2', 'status': 'FAIL' if unresolved else 'PASS',
            'task_count': len(tasks), 'product_count': len(owners), 'ledgers': summaries,
            'requirement_count': len(requirements), 'scenario_count': sum(map(len, requirements.values())),
            'acceptance_case_count': len(cases),
            'unresolved_dependencies': unresolved,
            'unresolved_dependency_ids': sorted({item['dependency'] for item in unresolved}),
            'limitation': 'Inventory consistency only; evidence content, semantic completeness and acceptance require independent review.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repositories-root', required=True, type=Path)
    parser.add_argument('--inventory', required=True, type=Path)
    parser.add_argument('--audit', action='store_true', help='List all unresolved dependency references and native statistics; unresolved inventory still exits 1.')
    args = parser.parse_args()
    try:
        require(args.inventory.is_absolute(), 'inventory must be absolute')
        # Check every ancestor of the supplied inventory before reading it.
        inventory_path = safe_path(Path(args.inventory.anchor), args.inventory.as_posix()[len(Path(args.inventory.anchor).as_posix()):].lstrip('/'))
        def no_duplicates(pairs):
            value = {}
            for key, item in pairs:
                require(key not in value, f'duplicate JSON key: {key}')
                value[key] = item
            return value
        data = json.loads(read_text(inventory_path), object_pairs_hook=no_duplicates)
        result = validate(args.repositories_root, data, audit=args.audit)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0 if result['status'] == 'PASS' else 1
    except (ValueError, OSError, RecursionError) as error:
        print(json.dumps({'status': 'FAIL', 'error': str(error)}, ensure_ascii=False), file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
