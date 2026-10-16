"""Exercise real Guard binaries in disposable directories, without changing host settings."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest

parser = argparse.ArgumentParser()
parser.add_argument('--bin-dir', type=Path, required=True)
parser.add_argument('--tools', default='archguard,codeguard,flowguard,gitguard,specguard,testguard')
parser.add_argument('--gitflow-root', type=Path)
config = parser.parse_args()
TOOLS = config.tools.split(',')
BIN = config.bin_dir.resolve()


class Installation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='guard-install-acceptance-')
        self.root = Path(self.temp.name)
        self.project = self.root / 'project with spaces'
        self.project.mkdir()
        self.home = self.root / 'isolated home'
        self.home.mkdir()

    def tearDown(self):
        self.temp.cleanup()

    def binary(self, tool):
        return BIN / (tool + ('.exe' if os.name == 'nt' else ''))

    def call(self, tool, *args, expected=0):
        out = subprocess.run([str(self.binary(tool)), *map(str, args), '--project', str(self.project),
                              '--home', str(self.home)], capture_output=True, timeout=45)
        self.assertEqual(out.returncode, expected, (tool, args, out.stdout.decode('utf-8'), out.stderr.decode('utf-8')))
        return json.loads(out.stdout)

    def snapshot(self):
        return {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob('*') if p.is_file()}

    def test_all_hosts_coexist_repeat_and_uninstall(self):
        human = b'# Human\r\n<!-- CODEGUARD:BEGIN -->\r\nExisting project context\r\n<!-- CODEGUARD:END -->\r\n'
        for name in ('AGENTS.md', 'CLAUDE.md', 'GEMINI.md'):
            (self.project / name).write_bytes(human)
        (self.project / '.codex').mkdir()
        (self.project / '.codex/config.toml').write_text('# keep comment\nmodel = "existing"\n[mcp_servers.other]\ncommand = "other-tool"\n', encoding='utf-8')
        (self.project / '.mcp.json').write_text(json.dumps({'custom': True, 'mcpServers': {'other': {'command': 'other-tool'}}}), encoding='utf-8')
        for tool in TOOLS:
            self.call(tool, 'install', '--agents', 'codex,claude,gemini')
            first = self.snapshot()
            self.assertEqual(self.call(tool, 'install')['changed_files'], [])
            self.assertEqual(first, self.snapshot())
            self.assertEqual(self.call(tool, 'doctor', '--integration')['status'], 'configured')
        toml = tomllib.loads((self.project / '.codex/config.toml').read_text(encoding='utf-8'))
        self.assertEqual(set(toml['mcp_servers']), {'other', *TOOLS})
        for tool in TOOLS:
            for name in ('AGENTS.md', 'CLAUDE.md', 'GEMINI.md'):
                self.assertEqual((self.project / name).read_text(encoding='utf-8').count(f'<!-- {tool.upper()}_START -->'), 1)
            self.call(tool, 'uninstall')
            for name in ('AGENTS.md', 'CLAUDE.md', 'GEMINI.md'):
                self.assertNotIn(f'<!-- {tool.upper()}_START -->', (self.project / name).read_text(encoding='utf-8'))
        for name in ('AGENTS.md', 'CLAUDE.md', 'GEMINI.md'):
            self.assertEqual((self.project / name).read_bytes(), human)
        self.assertEqual(tomllib.loads((self.project / '.codex/config.toml').read_text())['model'], 'existing')
        self.assertIn('# keep comment', (self.project / '.codex/config.toml').read_text())
        self.assertEqual(json.loads((self.project / '.mcp.json').read_text()), {'custom': True, 'mcpServers': {'other': {'command': 'other-tool'}}})

    def test_preview_and_user_scope_and_binary_lifecycle(self):
        for tool in TOOLS:
            with self.subTest(tool=tool):
                destination = self.root / ('bin-' + tool)
                initial = self.snapshot()
                self.call(tool, 'install', '--agents', 'codex,claude,gemini', '--scope', 'user', '--bin-dir', destination, '--dry-run')
                self.assertEqual(initial, self.snapshot())
                self.assertFalse(destination.exists())
                self.call(tool, 'install', '--agents', 'codex,claude,gemini', '--scope', 'user', '--bin-dir', destination)
                installed = destination / self.binary(tool).name
                self.assertEqual(hashlib.sha256(installed.read_bytes()).digest(), hashlib.sha256(self.binary(tool).read_bytes()).digest())
                result = subprocess.run([str(installed), 'context', '--project', str(self.project)], capture_output=True, timeout=45)
                self.assertEqual(result.returncode, 0, result.stdout)
                self.assertEqual(json.loads(result.stdout)['quality_decision'], 'not_evaluated')
                body = (self.home / '.codex/AGENTS.md').read_text(encoding='utf-8')
                self.assertIn('此用户级指引', body)
                self.assertNotIn('项目上下文（生成摘要', body)
                self.assertFalse((self.project / 'AGENTS.md').exists())
                self.call(tool, 'uninstall', '--scope', 'user')
                self.assertTrue(installed.is_file())
                initial = self.snapshot()
                self.call(tool, 'uninstall', '--scope', 'user', '--remove-binary', expected=4)
                self.assertEqual(initial, self.snapshot())
                self.call(tool, 'uninstall', '--scope', 'user', '--remove-binary', '--bin-dir', destination)
                self.assertFalse(installed.exists())

    def test_conflicts_do_not_partially_write(self):
        tool = TOOLS[0]
        (self.project / '.mcp.json').write_text(json.dumps({'mcpServers': {tool: {'command': 'user-command'}}}))
        initial = self.snapshot()
        self.call(tool, 'install', '--agents', 'codex,claude,gemini', expected=4)
        self.assertEqual(initial, self.snapshot())
        (self.project / '.mcp.json').unlink()
        block = f'<!-- {tool.upper()}_START -->broken'
        (self.project / 'AGENTS.md').write_text(block)
        initial = self.snapshot()
        self.call(tool, 'install', '--agents', 'codex', expected=4)
        self.assertEqual(initial, self.snapshot())
        (self.project / 'AGENTS.md').unlink()
        self.call(tool, 'install', '--agents', 'codex')
        path = self.project / 'AGENTS.md'
        path.write_text(path.read_text(encoding='utf-8').replace('## ', '## Edited ', 1), encoding='utf-8')
        initial = self.snapshot()
        for args in [('install',), ('instructions', 'sync'), ('uninstall',)]:
            self.call(tool, *args, expected=4)
            self.assertEqual(initial, self.snapshot())

    def test_mcp_protocol_and_cli_context_match(self):
        before = self.snapshot()
        for tool in TOOLS:
            messages = [
                {'jsonrpc': '2.0', 'id': 0, 'method': 'tools/list'},
                {'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2024-11-05', 'capabilities': {}, 'clientInfo': {'name': 'acceptance', 'version': '1'}}},
                {'jsonrpc': '2.0', 'method': 'notifications/initialized'},
                {'jsonrpc': '2.0', 'id': 2, 'method': 'tools/list'},
                {'jsonrpc': '2.0', 'id': 3, 'method': 'tools/call', 'params': {'name': tool+'_context', 'arguments': {'project': str(self.project)}}},
                {'jsonrpc': '2.0', 'id': 4, 'method': 'tools/call', 'params': {'name': tool+'_instructions', 'arguments': {'project': str(self.project)}}},
                {'jsonrpc': '2.0', 'id': 5, 'method': 'tools/call', 'params': {'name': tool+'_context', 'arguments': {'project': '.'}}},
            ]
            result = subprocess.run([str(self.binary(tool)), 'serve', '--mcp'], input=('\n'.join(map(json.dumps, messages))+'\n').encode(), capture_output=True, timeout=45)
            self.assertEqual(result.returncode, 0, result.stderr)
            replies = {row['id']: row for row in map(json.loads, result.stdout.splitlines())}
            self.assertEqual(replies[0]['error']['code'], -32000)
            self.assertEqual(replies[1]['result']['protocolVersion'], '2024-11-05')
            expected_tools = {tool+'_context', tool+'_instructions'}
            if tool == 'archguard':
                expected_tools.update({'archguard_check', 'archguard_explain', 'archguard_verify'})
                expected_tools.update({'archguard_model_show', 'archguard_model_validate', 'archguard_change_impact', 'archguard_change_check', 'archguard_change_verify'})
            if tool == 'specguard':
                expected_tools.update({'specguard_check_agents', 'specguard_agents_template'})
                expected_tools.update({'specguard_check_specs', 'specguard_verify_specs'})
                expected_tools.update({'specguard_check_trace', 'specguard_verify_trace'})
            if tool == 'gitguard':
                expected_tools.add('gitguard_policy_check')
            if tool == 'testguard':
                expected_tools.add('testguard_verify_junit')
                expected_tools.add('testguard_verify_execution')
            if tool == 'flowguard':
                expected_tools.add('flowguard_stage_check')
                expected_tools.add('flowguard_evidence_check')
            self.assertEqual({x['name'] for x in replies[2]['result']['tools']}, expected_tools)
            context = json.loads(replies[3]['result']['content'][0]['text'])
            self.assertEqual(context, self.call(tool, 'context'))
            self.assertIn(f'<!-- {tool.upper()}_START -->', replies[4]['result']['content'][0]['text'])
            self.assertEqual(replies[5]['error']['code'], -32602)
        self.assertEqual(before, self.snapshot())

    def test_untrusted_receipt_does_not_execute_adapter(self):
        if 'gitguard' not in TOOLS:
            self.skipTest('gitguard not selected')
        self.call('gitguard', 'install', '--agents', 'codex')
        receipt_path = self.project / '.guard-install/gitguard.json'
        receipt = json.loads(receipt_path.read_text(encoding='utf-8'))
        receipt['gitflow_root'] = str(self.root / 'untrusted plugin')
        receipt['python'] = str(self.root / 'untrusted executable')
        receipt_path.write_text(json.dumps(receipt))
        self.assertEqual(self.call('gitguard', 'context')['domain']['status'], 'unconfigured')
        initial = self.snapshot()
        self.call('gitguard', 'instructions', 'sync', expected=4)
        self.assertEqual(initial, self.snapshot())

    def test_unknown_options_locks_and_binary_ownership(self):
        tool = TOOLS[0]
        self.call(tool, 'install', '--agents', 'codex', '--unexpected', 'value', expected=4)
        self.assertEqual(self.snapshot(), {})
        state = self.project / '.guard-install'
        state.mkdir()
        lock = state / '.lock'
        lock.write_text('another installation')
        initial = self.snapshot()
        self.call(tool, 'install', '--agents', 'codex', expected=4)
        self.assertEqual(initial, self.snapshot())
        lock.unlink()
        destination = self.root / 'bin'
        destination.mkdir()
        binary = destination / self.binary(tool).name
        binary.write_bytes(b'not owned by installer')
        initial = self.snapshot()
        self.call(tool, 'install', '--agents', 'codex', '--bin-dir', destination, expected=4)
        self.assertEqual(initial, self.snapshot())

    def test_domain_capability_boundaries_are_truthful(self):
        for tool in TOOLS:
            context = self.call(tool, 'context')
            self.assertEqual(context['quality_decision'], 'not_evaluated')
            if not context['domain_commands']:
                result = subprocess.run([str(self.binary(tool)), 'check', '--project', str(self.project)], capture_output=True, timeout=15)
                self.assertEqual(result.returncode, 4, (tool, result.stdout, result.stderr))
                self.assertNotIn(b'PASS', result.stdout)
            if tool == 'codeguard' and not context['domain_commands']:
                version = subprocess.run([str(self.binary(tool)), '--version'], capture_output=True, timeout=15)
                self.assertIn(b'agent integration only', version.stdout)

    def test_six_installed_blocks_pass_specguard_and_sync_preserves_human_text(self):
        if not {'archguard', 'codeguard', 'flowguard', 'gitguard', 'specguard', 'testguard'} <= set(TOOLS):
            self.skipTest('all six tools required')
        human = '# Project instructions\nPreserve this custom convention.\n'
        (self.project / 'AGENTS.md').write_text(human, encoding='utf-8')
        for tool in TOOLS:
            self.call(tool, 'install', '--agents', 'codex')
        result = subprocess.run([str(self.binary('specguard')), 'agents', 'check', '--project', str(self.project)], capture_output=True, timeout=45)
        self.assertEqual(result.returncode, 0, result.stdout.decode('utf-8'))
        self.assertEqual(json.loads(result.stdout)['execution_status'], 'not_evaluated')
        for tool in TOOLS:
            self.call(tool, 'instructions', 'sync')
        self.assertTrue((self.project / 'AGENTS.md').read_text(encoding='utf-8').startswith(human))

    def test_linked_host_configuration_is_rejected(self):
        target = self.root / 'external config'
        target.mkdir()
        (target / 'config.toml').write_text('model = "preserve"')
        link = self.project / '.codex'
        if os.name == 'nt':
            result = subprocess.run(['cmd', '/c', 'mklink', '/J', str(link), str(target)], capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            link.symlink_to(target, target_is_directory=True)
        try:
            self.call(TOOLS[0], 'install', '--agents', 'codex', expected=4)
            self.assertEqual((target / 'config.toml').read_text(), 'model = "preserve"')
            self.assertFalse((self.project / 'AGENTS.md').exists())
        finally:
            if os.name == 'nt':
                link.rmdir()
            else:
                link.unlink()

    def test_gitflow_profiles_drift_and_refresh(self):
        if 'gitguard' not in TOOLS or not config.gitflow_root:
            self.skipTest('explicit existing GitFlow source required')
        plugin = config.gitflow_root.resolve()
        adapter = ('--gitflow-root', plugin, '--python', Path(sys.executable).resolve())
        subprocess.run(['git', 'init', '--quiet', str(self.project)], check=True, capture_output=True)
        # This disposable fixture checks policy selection, not host-installed clean filters.
        # GitGuard deliberately refuses executable filters before querying status.
        filters = subprocess.run(['git', '-C', str(self.project), 'config', '--name-only',
                                  '--get-regexp', r'^filter\..*\.(clean|process)$'],
                                 capture_output=True, text=True, check=False)
        for key in filters.stdout.splitlines():
            subprocess.run(['git', '-C', str(self.project), 'config', '--local', key, ''],
                           check=True, capture_output=True)
        state = self.project / '.gitflow/state'
        state.mkdir(parents=True)

        def activate(policy):
            digest = hashlib.sha256(json.dumps(policy, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
            (state / 'activation.json').write_text(json.dumps({'schema_version': '1.0.0', 'mode': 'shared', 'policy': policy, 'sha256': digest}), encoding='utf-8')
            (self.project / '.gitflow/workflow.json').write_text(json.dumps(policy), encoding='utf-8')

        github = json.loads((plugin / 'profiles/github-flow.json').read_text())
        classic = json.loads((plugin / 'profiles/classic-gitflow.json').read_text())
        for policy in (github, classic):
            activate(policy)
            context = self.call('gitguard', 'context', *adapter)
            self.assertEqual(context['domain']['status'], 'active', context)
            self.assertEqual(context['domain']['roles'], policy['roles'])
            self.call('gitguard', 'install', '--agents', 'codex,claude,gemini', *adapter)
            text = (self.project / 'AGENTS.md').read_text(encoding='utf-8')
            self.assertIn(policy['profile'], text)
            self.assertEqual(self.call('gitguard', 'doctor', '--integration', *adapter)['status'], 'configured')
        self.assertIn('develop', text)
        self.assertNotIn('github-flow', text)
        classic['revision'] += 1
        (self.project / '.gitflow/workflow.json').write_text(json.dumps(classic))
        self.assertEqual(self.call('gitguard', 'context', *adapter)['domain']['code'], 'policy_drift')
        self.call('gitguard', 'doctor', '--integration', *adapter, expected=3)
        activate(classic)
        self.call('gitguard', 'instructions', 'sync', *adapter)
        self.assertEqual(self.call('gitguard', 'doctor', '--integration', *adapter)['status'], 'configured')
        # The generated MCP command pins trusted adapter paths rather than taking them from queried repositories.
        entry = tomllib.loads((self.project / '.codex/config.toml').read_text(encoding='utf-8'))['mcp_servers']['gitguard']
        messages = [
            {'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'test','version':'1'}}},
            {'jsonrpc':'2.0','method':'notifications/initialized'},
            {'jsonrpc':'2.0','id':2,'method':'tools/call','params':{'name':'gitguard_context','arguments':{'project':str(self.project)}}},
        ]
        result = subprocess.run([entry['command'], *entry['args']], input=('\n'.join(map(json.dumps,messages))+'\n').encode(), capture_output=True, timeout=45)
        self.assertEqual(result.returncode, 0, result.stderr)
        reply = json.loads(result.stdout.splitlines()[-1])
        self.assertEqual(json.loads(reply['result']['content'][0]['text'])['domain']['status'], 'active')


if __name__ == '__main__':
    unittest.main(argv=[sys.argv[0]], verbosity=2)
