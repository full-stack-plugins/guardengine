"""Delivery inventory rejects structural loss; no semantic completion inference."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / 'verify_delivery_inventory.py'
spec = importlib.util.spec_from_file_location('delivery_inventory', MODULE)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class InventoryTests(unittest.TestCase):
    def setUp(self):
        base = Path(os.environ.get('TEMP', tempfile.gettempdir()))
        self.temp = tempfile.TemporaryDirectory(dir=base)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.repo = self.root / 'engine'
        self.repo.mkdir()
        self.ledger = self.repo / 'tasks.md'
        self.ledger.write_text('- [ ] **EG-M01** inventory 依赖：无。规格：EG-PROD-001。\n'
                               '- [ ] **EG-K01** protocol 依赖：EG-M01。规格：EG-CON-001。\n', encoding='utf-8')
        (self.repo / 'ownership.md').write_text('| EG-M01 | `guardcore` | inventory |\n| EG-K01 | `guardcore` | protocol |\n', encoding='utf-8')
        self.specification = self.repo / 'spec.md'
        self.specification.write_text('### Requirement: EG-PROD-001 Product boundary\n\nSystem SHALL preserve owners.\n\n'
                                      '#### Scenario: owner preserved\n- **WHEN** a task is assigned\n- **THEN** its native owner is retained\n\n'
                                      '### Requirement: EG-CON-001 Contract\n\nSystem SHALL retain snapshots.\n\n'
                                      '#### Scenario: rules changed\n- **WHEN** a candidate changes rules\n- **THEN** approved rules remain active\n', encoding='utf-8')
        (self.repo / 'acceptance.md').write_text('| EG-AT-001 | delivery | real evidence required | EG-M01, EG-K01 |\n', encoding='utf-8')
        self.data = {'schema_version': 'guard.delivery-inventory/v2',
                     'products': [{'id': 'guardcore', 'repository': 'engine', 'logical_names': ['guardengine']}],
                     'ownership': {'repository': 'engine', 'path': 'ownership.md'},
                     'specifications': [{'repository': 'engine', 'path': 'spec.md', 'requirements': [
                         {'id': 'EG-PROD-001', 'scenarios': ['owner preserved']},
                         {'id': 'EG-CON-001', 'scenarios': ['rules changed']}]}],
                     'acceptance': [{'repository': 'engine', 'path': 'acceptance.md', 'cases': [
                         {'id': 'EG-AT-001', 'tasks': ['EG-M01', 'EG-K01']}]}],
                     'ledgers': [{'repository': 'engine', 'path': 'tasks.md', 'ledger_kind': 'stable_ids'}],
                     'tasks': [self.task('EG-M01', [], ['EG-PROD-001']),
                               self.task('EG-K01', ['EG-M01'], ['EG-CON-001'])]}

    def task(self, identity, dependencies, requirements):
        return {'id': identity, 'owner': 'guardcore',
                'ledger': {'repository': 'engine', 'path': 'tasks.md'},
                'requirements': requirements, 'dependencies': dependencies,
                'evidence': [], 'status': 'pending'}

    def check(self):
        return checker.validate(self.root, self.data)

    def test_valid_and_native_stats(self):
        (self.repo / 'native.md').write_text('- [x] existing\n- [ ] remaining\n', encoding='utf-8')
        self.data['ledgers'].append({'repository': 'engine', 'path': 'native.md', 'ledger_kind': 'native_tasks'})
        result = self.check()
        self.assertEqual(2, result['task_count'])
        self.assertEqual(1, result['ledgers'][1]['completed'])

    def test_missing_duplicate_unknown_and_changed_references(self):
        for mutation in ['missing', 'duplicate', 'dependency', 'requirement', 'owner']:
            with self.subTest(mutation=mutation):
                original = json.loads(json.dumps(self.data))
                if mutation == 'missing': self.data['tasks'].pop()
                elif mutation == 'duplicate': self.data['tasks'].append(dict(self.data['tasks'][0]))
                elif mutation == 'dependency': self.data['tasks'][1]['dependencies'] = ['EG-Z99']
                elif mutation == 'requirement': self.data['tasks'][0]['requirements'] = []
                else: self.data['tasks'][0]['owner'] = 'unknown'
                with self.assertRaises(ValueError): self.check()
                self.data = original

    def test_cycle(self):
        self.ledger.write_text('- [ ] **EG-M01** 依赖：EG-K01。规格：EG-PROD-001。\n'
                               '- [ ] **EG-K01** 依赖：EG-M01。规格：EG-CON-001。\n', encoding='utf-8')
        self.data['tasks'][0]['dependencies'] = ['EG-K01']
        with self.assertRaisesRegex(ValueError, 'cycle'): self.check()

    def test_verified_requires_native_completion_and_evidence(self):
        self.data['tasks'][0]['status'] = 'verified'
        with self.assertRaises(ValueError): self.check()
        (self.repo / 'evidence.txt').write_text('actual evidence', encoding='utf-8')
        self.data['tasks'][0]['evidence'] = [{'repository': 'engine', 'path': 'evidence.txt'}]
        with self.assertRaises(ValueError): self.check()
        self.ledger.write_text(self.ledger.read_text(encoding='utf-8').replace('- [ ] **EG-M01**', '- [x] **EG-M01**'), encoding='utf-8')
        self.assertEqual(2, self.check()['task_count'])

    def test_paths_aliases_and_schema_rejected(self):
        for mutation in ['escape', 'absolute', 'alias', 'unknown_field']:
            with self.subTest(mutation=mutation):
                original = json.loads(json.dumps(self.data))
                if mutation == 'escape': self.data['ledgers'][0]['path'] = '../tasks.md'
                elif mutation == 'absolute': self.data['ledgers'][0]['path'] = str(self.ledger)
                elif mutation == 'alias': self.data['products'].append({'id': 'other', 'repository': 'engine', 'logical_names': []})
                else: self.data['tasks'][0]['mystery'] = True
                with self.assertRaises(ValueError): self.check()
                self.data = original

    def test_duplicate_native_id_and_unlisted_ledger(self):
        self.ledger.write_text(self.ledger.read_text(encoding='utf-8') * 2, encoding='utf-8')
        with self.assertRaises(ValueError): self.check()

    def test_unlisted_active_ledger_is_not_silently_lost(self):
        path = self.repo / 'openspec/changes/new-work'
        path.mkdir(parents=True)
        (path / 'tasks.md').write_text('- [ ] not yet implemented\n', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'unlisted active ledgers'): self.check()

    def test_link_ledger_rejected(self):
        link = self.repo / 'link.md'
        try:
            link.symlink_to(self.ledger)
        except OSError:
            self.skipTest('host does not grant symlink creation')
        self.data['ledgers'][0]['path'] = 'link.md'
        with self.assertRaisesRegex(ValueError, 'link/reparse'): self.check()

    def test_native_owner_and_kind_cannot_be_reassigned(self):
        original = self.ledger.read_text(encoding='utf-8')
        self.ledger.write_text(original.replace('**EG-M01**', '**EG-M01** [owner=other]'), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'native owner differs'): self.check()
        self.ledger.write_text(original, encoding='utf-8')
        self.data['ledgers'][0]['ledger_kind'] = 'native_tasks'
        with self.assertRaisesRegex(ValueError, 'stable IDs cannot be hidden'): self.check()

    def test_completed_checkbox_never_automatically_promotes_status(self):
        self.ledger.write_text(self.ledger.read_text(encoding='utf-8').replace('[ ]', '[x]'), encoding='utf-8')
        self.check()
        self.assertEqual(['pending', 'pending'], [task['status'] for task in self.data['tasks']])

    def test_audit_reports_every_unknown_dependency_without_passing(self):
        self.ledger.write_text('- [ ] **EG-M01** inventory 依赖：EG-F01, EG-F02。规格：EG-PROD-001。\n'
                               '- [ ] **EG-K01** protocol 依赖：EG-R01。规格：EG-CON-001。\n', encoding='utf-8')
        self.data['tasks'][0]['dependencies'] = ['EG-F01', 'EG-F02']
        self.data['tasks'][1]['dependencies'] = ['EG-R01']
        with self.assertRaisesRegex(ValueError, 'unknown dependencies'): self.check()
        result = checker.validate(self.root, self.data, audit=True)
        self.assertEqual('FAIL', result['status'])
        self.assertEqual(3, len(result['unresolved_dependencies']))
        self.assertEqual(2, result['ledgers'][0]['total'])

    def test_body_task_references_are_not_duplicate_definitions(self):
        self.ledger.write_text(self.ledger.read_text(encoding='utf-8').replace('inventory ', 'inventory delegates EG-K01; '), encoding='utf-8')
        self.assertEqual(2, self.check()['task_count'])

    def test_noncanonical_paths_and_duplicate_physical_owner_rejected(self):
        for relative in ['engine/', 'engine//']:
            original = json.loads(json.dumps(self.data))
            self.data['products'].append({'id': 'other', 'repository': relative, 'logical_names': []})
            with self.assertRaises(ValueError): self.check()
            self.data = original
        self.data['ledgers'].append({'repository': 'engine', 'path': 'tasks.md/', 'ledger_kind': 'stable_ids'})
        with self.assertRaises(ValueError): self.check()

    def test_authoritative_ownership_cannot_be_changed_in_manifest(self):
        other = self.root / 'other'
        other.mkdir()
        self.data['products'].append({'id': 'other', 'repository': 'other', 'logical_names': []})
        self.data['tasks'][1]['owner'] = 'other'
        with self.assertRaisesRegex(ValueError, 'authoritative owner'): self.check()

    def test_empty_segments_and_windows_physical_aliases(self):
        docs = self.repo / 'docs'
        docs.mkdir()
        (docs / 'native.md').write_text('- [ ] native\n', encoding='utf-8')
        self.data['ledgers'].extend([
            {'repository': 'engine', 'path': 'docs/native.md', 'ledger_kind': 'native_tasks'},
            {'repository': 'engine', 'path': 'docs//native.md', 'ledger_kind': 'native_tasks'}])
        with self.assertRaisesRegex(ValueError, 'unsafe path'): self.check()
        self.data['ledgers'].pop()
        if os.name == 'nt':
            self.data['ledgers'].append({'repository': 'engine', 'path': 'DOCS/native.md', 'ledger_kind': 'native_tasks'})
            with self.assertRaisesRegex(ValueError, 'duplicate physical ledger'): self.check()
            self.data['ledgers'].pop()
            self.data['products'].append({'id': 'other', 'repository': 'ENGINE', 'logical_names': []})
            with self.assertRaisesRegex(ValueError, 'duplicate physical repository'): self.check()

    def test_scenario_inventory_cannot_lose_names_or_native_behaviors(self):
        self.data['specifications'][0]['requirements'][0]['scenarios'] = []
        with self.assertRaisesRegex(ValueError, 'scenario inventory differs'): self.check()
        self.data['specifications'][0]['requirements'][0]['scenarios'] = ['owner preserved']
        self.specification.write_text(self.specification.read_text(encoding='utf-8').replace('- **THEN** its native owner is retained', '- incomplete'), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'WHEN/THEN'): self.check()

    def test_requirement_and_acceptance_sources_are_authoritative(self):
        self.data['specifications'][0]['requirements'].pop()
        with self.assertRaisesRegex(ValueError, 'requirement inventory differs'): self.check()
        self.data['specifications'][0]['requirements'].append({'id': 'EG-CON-001', 'scenarios': ['rules changed']})
        self.data['acceptance'][0]['cases'][0]['tasks'] = ['EG-M01']
        with self.assertRaisesRegex(ValueError, 'acceptance task inventory differs'): self.check()

    def test_unknown_requirement_and_case_tasks_rejected(self):
        text = self.ledger.read_text(encoding='utf-8').replace('EG-CON-001', 'EG-ABSENT-001')
        self.ledger.write_text(text, encoding='utf-8')
        self.data['tasks'][1]['requirements'] = ['EG-ABSENT-001']
        with self.assertRaisesRegex(ValueError, 'unknown requirements'): self.check()
        self.ledger.write_text(text.replace('EG-ABSENT-001', 'EG-CON-001'), encoding='utf-8')
        self.data['tasks'][1]['requirements'] = ['EG-CON-001']
        (self.repo / 'acceptance.md').write_text('| EG-AT-001 | delivery | evidence | EG-Z99 |\n', encoding='utf-8')
        self.data['acceptance'][0]['cases'][0]['tasks'] = ['EG-Z99']
        with self.assertRaisesRegex(ValueError, 'unknown acceptance tasks'): self.check()

    def test_code_fences_cannot_supply_fake_requirements_or_scenarios(self):
        text = self.specification.read_text(encoding='utf-8')
        fake = '\n```markdown\n### Requirement: EG-FAKE-001 Fake\n#### Scenario: fake\n- **WHEN** fake\n- **THEN** fake\n```\n'
        self.specification.write_text(text + fake, encoding='utf-8')
        self.assertEqual(2, self.check()['requirement_count'])
        self.specification.write_text(text + '\n```\nunclosed', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'unclosed code fence'): self.check()

    def test_duplicate_requirement_scenario_and_acceptance_rejected(self):
        original = self.specification.read_text(encoding='utf-8')
        self.specification.write_text(original * 2, encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'duplicate requirement'): self.check()
        self.specification.write_text(original + '\n#### Scenario: rules changed\n- **WHEN** x\n- **THEN** y\n', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'duplicate scenario'): self.check()
        self.specification.write_text(original, encoding='utf-8')
        acceptance = self.repo / 'acceptance.md'
        acceptance.write_text(acceptance.read_text(encoding='utf-8') * 2, encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'duplicate acceptance'): self.check()

    def test_fenced_examples_cannot_become_task_or_owner_definitions(self):
        example = '\n```markdown\n- [x] **EG-Z99** example 依赖：无。规格：EG-CON-001。\n```\n'
        self.ledger.write_text(self.ledger.read_text(encoding='utf-8') + example, encoding='utf-8')
        ownership = self.repo / 'ownership.md'
        ownership.write_text(ownership.read_text(encoding='utf-8') + '\n```\n| EG-Z99 | `guardcore` | example |\n```\n', encoding='utf-8')
        result = self.check()
        self.assertEqual(2, result['task_count'])
        self.assertEqual(2, result['ledgers'][0]['total'])

    def test_missing_end_to_end_case_is_not_a_complete_inventory(self):
        self.data['acceptance'][0]['cases'] = []
        with self.assertRaisesRegex(ValueError, 'acceptance case inventory differs'): self.check()


if __name__ == '__main__':
    unittest.main()
