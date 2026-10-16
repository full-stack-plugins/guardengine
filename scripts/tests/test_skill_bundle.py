"""公共技能构建的黑盒验收；仅使用标准库和隔离目录。"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

BUILDER = Path(__file__).resolve().parents[1] / 'skill_bundle.py'
LOCKER = BUILDER.with_name('lock_skill_library.py')


class SkillBundle(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='guard-skills-')
        self.root = Path(self.temp.name)
        self.library = self.root / 'library'
        self.plugin = self.root / 'plugin'
        self.library.mkdir()
        self.plugin.mkdir()
        self.lock = {'schema_version': 1, 'upstream': {'repository': 'https://github.com/mattpocock/skills', 'revision': 'a' * 40, 'version': '1.3.1'}, 'files': {}, 'skills': {}}
        for name, dependencies in [('archguard-grill-with-docs', ['guard-grilling']), ('guard-grilling', [])]:
            path = f'adapters/{name}/SKILL.md'
            self.file(path, f'---\nname: {name}\nlicense: MIT\ndescription: Test method\n---\nMethod: {dependencies}\n')
            self.lock['skills'][name] = {'path': f'adapters/{name}', 'dependencies': dependencies}
        self.file('sources/LICENSE', 'MIT test notice')
        (self.plugin / 'plugin.json').write_text('{"name":"fixture"}', encoding='utf-8')
        (self.plugin / 'skills.request.json').write_text(json.dumps({'schema_version': 1, 'skills': ['archguard-grill-with-docs']}), encoding='utf-8')
        self.save()

    def tearDown(self):
        self.temp.cleanup()

    def file(self, name, content):
        path = self.library / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content.encode('utf-8'))
        self.lock['files'][name] = hashlib.sha256(path.read_bytes()).hexdigest()

    def save(self):
        (self.library / 'skills.lock.json').write_text(json.dumps(self.lock), encoding='utf-8')

    def run_bundle(self, *args, code=0):
        result = subprocess.run([sys.executable, '-B', str(BUILDER), '--library', str(self.library), '--plugin', str(self.plugin), *args], capture_output=True, text=True, encoding='utf-8')
        self.assertEqual(result.returncode, code, result.stdout + result.stderr)
        if code:
            self.assertEqual(json.loads(result.stderr)['status'], 'error')
        return result

    def snapshot(self):
        return {p.relative_to(self.plugin).as_posix(): p.read_bytes() for p in self.plugin.rglob('*') if p.is_file()}

    def test_dependency_closure_and_idempotent_offline_check(self):
        self.run_bundle()
        self.assertTrue((self.plugin / 'skills/guard-grilling/SKILL.md').is_file())
        before = self.snapshot()
        self.run_bundle('--check')
        self.run_bundle()
        self.assertEqual(before, self.snapshot())

    def test_lock_rejects_crlf_before_replacing_reviewed_lock(self):
        catalog = {key: value for key, value in self.lock.items() if key != 'files'}
        (self.library / 'catalog.json').write_bytes(json.dumps(catalog).encode('utf-8'))
        path = self.library / 'adapters/guard-grilling/SKILL.md'
        original = path.read_bytes()
        path.write_bytes(original.replace(b'\n', b'\r\n'))
        before = (self.library / 'skills.lock.json').read_bytes()
        result = subprocess.run([sys.executable, '-B', str(LOCKER), '--library', str(self.library)], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('LF line endings required', result.stderr)
        self.assertEqual(before, (self.library / 'skills.lock.json').read_bytes())
        path.write_bytes(original)
        subprocess.run([sys.executable, '-B', str(LOCKER), '--library', str(self.library)], check=True, capture_output=True)
        self.run_bundle()
        self.run_bundle('--check')
        # 校验继续比较原始字节，不把换行篡改隐式归一化为可信输入。
        path.write_bytes(original.replace(b'\n', b'\r\n'))
        result = self.run_bundle('--check', code=2)
        self.assertIn('locked source digest mismatch', result.stderr)

    def test_missing_dependency_cycle_and_source_digest_fail_before_writing(self):
        for dependencies in [['absent'], ['archguard-grill-with-docs']]:
            self.lock['skills']['guard-grilling']['dependencies'] = dependencies
            self.save()
            before = self.snapshot()
            self.run_bundle(code=2)
            self.assertEqual(before, self.snapshot())
        self.lock['skills']['guard-grilling']['dependencies'] = []
        self.save()
        (self.library / 'sources/LICENSE').write_text('tampered', encoding='utf-8')
        self.run_bundle(code=2)

    def test_preserve_manual_output_and_unmanaged_collisions(self):
        self.run_bundle()
        path = self.plugin / 'skills/guard-grilling/SKILL.md'
        path.write_text('human edit', encoding='utf-8')
        before = self.snapshot()
        self.run_bundle(code=2)
        self.run_bundle('--check', code=2)
        self.assertEqual(before, self.snapshot())

    def test_path_escape_and_empty_request_rejected(self):
        self.lock['skills']['guard-grilling']['path'] = '../outside'
        self.save()
        self.run_bundle(code=2)
        self.lock['skills']['guard-grilling']['path'] = 'adapters/guard-grilling'
        self.save()
        (self.plugin / 'skills.request.json').write_text('{"schema_version":1,"skills":[]}', encoding='utf-8')
        self.run_bundle(code=2)

    def test_missing_reference_and_unmanaged_directory_rejected(self):
        self.file('adapters/guard-grilling/SKILL.md', '---\nname: guard-grilling\nlicense: MIT\ndescription: Test\n---\n[missing](references/missing.md)\n')
        self.save()
        result = self.run_bundle(code=2)
        self.assertIn('missing packaged reference', result.stderr)
        self.file('adapters/guard-grilling/SKILL.md', '---\nname: guard-grilling\nlicense: MIT\ndescription: Test\n---\nMethod\n')
        self.save()
        directory = self.plugin / 'skills/guard-grilling'
        directory.mkdir(parents=True)
        (directory / 'human.md').write_text('keep', encoding='utf-8')
        before = self.snapshot()
        self.run_bundle(code=2)
        self.assertEqual(before, self.snapshot())


if __name__ == '__main__':
    unittest.main()
