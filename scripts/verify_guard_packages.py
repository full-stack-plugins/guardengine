"""Check local portable package structure and real stdio discovery; not host installation proof."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess


def check(root, binaries, tool):
    package = (root / tool / 'plugin').resolve(strict=True)
    manifest = json.loads((package / 'plugin.json').read_text(encoding='utf-8'))
    assert manifest['$schema'] == 'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json'
    assert manifest['name'] == tool
    assert not (set(manifest) - {'$schema', 'name', 'version', 'description', 'author',
                                 'homepage', 'repository', 'license', 'keywords', 'extensions'})
    config = json.loads((package / 'mcp.json').read_text(encoding='utf-8'))
    assert config['$schema'] == 'https://agent-plugins.org/schemas/1.0.0/mcp.schema.json'
    assert set(config) == {'$schema', 'mcpServers'}
    server = config['mcpServers'][tool]
    assert server == {'type': 'stdio', 'command': tool, 'args': ['serve', '--mcp']}
    skills = sorted((package / 'skills').glob('*/SKILL.md'))
    assert skills, f'{tool}: no skills'
    for skill in skills:
        assert skill.resolve().is_relative_to(package)
        source = skill.read_text(encoding='utf-8')
        assert source.startswith('---\n')
        front = source.split('---', 2)[1]
        assert re.search(r'^name:\s*' + re.escape(skill.parent.name) + r'\s*$', front, re.M)
        assert re.search(r'^description:\s*\S', front, re.M)
        for target in re.findall(r'\]\(([^)]+)\)', source):
            target = target.split('#', 1)[0]
            if not target or re.match(r'^[a-z]+:', target):
                continue
            path = (skill.parent / target).resolve(strict=True)
            assert path.is_relative_to(package), f'{tool}: external skill reference {target}'
    messages = [
        {'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {
            'protocolVersion': '2024-11-05', 'capabilities': {},
            'clientInfo': {'name': 'package-verification', 'version': '1'}}},
        {'jsonrpc': '2.0', 'method': 'notifications/initialized'},
        {'jsonrpc': '2.0', 'id': 2, 'method': 'tools/list'},
    ]
    binary = binaries / (tool + ('.exe' if os.name == 'nt' else ''))
    output = subprocess.run([str(binary), *server['args']],
                            input=''.join(json.dumps(m) + '\n' for m in messages),
                            capture_output=True, encoding='utf-8', timeout=30, check=True)
    replies = {r['id']: r for r in map(json.loads, output.stdout.splitlines()) if 'id' in r}
    assert replies[1]['result']['protocolVersion'] == '2024-11-05'
    names = [t['name'] for t in replies[2]['result']['tools']]
    assert len(names) == len(set(names)) and len(names) > 2
    return {'tool': tool, 'skills': len(skills), 'mcp_tools': len(names),
            'structure': 'checked', 'stdio': 'verified', 'host_loaded': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repositories-root', type=Path, required=True)
    parser.add_argument('--bin-dir', type=Path, required=True)
    parser.add_argument('--tools', default='archguard,gitguard,flowguard,specguard,testguard')
    args = parser.parse_args()
    print(json.dumps([check(args.repositories_root.resolve(), args.bin_dir.resolve(), tool)
                      for tool in args.tools.split(',')], ensure_ascii=False, indent=2))
