"""维护者审查 catalog/来源/适配后显式更新锁；构建与 CI 不自动运行本命令。"""
import argparse
import json
from pathlib import Path
from skill_bundle import digest, encoded, load, read, replace, safe


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--library', type=Path, required=True)
    args = parser.parse_args()
    root = args.library.resolve()
    catalog = load(safe(root, 'catalog.json'))
    files = {}
    for directory in ['sources', 'adapters', 'references']:
        for path in sorted(safe(root, directory).rglob('*')):
            relative = path.relative_to(root).as_posix()
            safe(root, relative)
            if path.is_file():
                content = read(path)
                # .gitattributes 会以 LF 提交；拒绝对尚未规范化的工作树生成无法复验的锁。
                if b'\r\n' in content:
                    raise ValueError(f'LF line endings required before locking: {relative}')
                files[relative] = digest(content)
    catalog['files'] = files
    replace(safe(root, 'skills.lock.json'), encoded(catalog))
    print(json.dumps({'status': 'locked', 'files': len(files), 'revision': catalog['upstream']['revision']}))


if __name__ == '__main__':
    main()
