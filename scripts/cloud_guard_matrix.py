#!/usr/bin/env python3
"""Run a declared local checkout slice; does not fetch, publish or configure host protection."""
import argparse
import json
import subprocess
from pathlib import Path
REPOSITORIES = ('guardengine','specguard','archguard','codeguard','testguard','gitguard','flowguard')
def commands(repo, root):
    manifest = root / repo / 'Cargo.toml'
    base = ['cargo', '+1.99.0', 'test', '--locked', '--manifest-path', str(manifest)]
    if repo == 'codeguard':
        tests = sorted((root / repo / 'crates/codeguard-cli/tests').glob('guard_integration_*.rs'))
        if not tests:
            raise ValueError('CodeGuard integration profile unavailable at selected revision')
        base += ['-p','codeguard-cli']
        for test in tests:
            base += ['--test', test.stem]
    return [base]
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--repository', choices=REPOSITORIES, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--run', action='store_true')
    args = parser.parse_args()
    selected = commands(args.repository, args.root.resolve())
    print(json.dumps({'repository':args.repository, 'scope':'CodeGuard integration targets only; other Guards full crate suite', 'commands': selected}, indent=2),flush=True)
    if args.run:
        for command in selected:
            subprocess.run(command,check=True)
if __name__ == '__main__':
    main()
