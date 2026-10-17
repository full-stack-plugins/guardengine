"""共享OPA CI准备器：受控字节流不代表真实OPA二进制资格。"""
import hashlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "prepare_ci_opa.py"


class PrepareCiOpa(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "shared CI preparation implementation is missing")
        spec = importlib.util.spec_from_file_location("prepare_ci_opa_under_test", SCRIPT)
        self.module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = self.module
        spec.loader.exec_module(self.module)
        self.temporary = tempfile.TemporaryDirectory(prefix="guard-ci-opa-")
        self.root = Path(self.temporary.name).resolve()
        self.directory = self.root / "state"
        self.directory.mkdir()
        self.output = self.directory / "opa-fixture"
        self.environment = patch.dict(os.environ, {
            "CI": "true", "GITHUB_ACTIONS": "true", "GITHUB_RUN_ID": "42",
            "RUNNER_TEMP": str(self.root),
        })
        self.environment.start()
        self.addCleanup(self.environment.stop)
        self.addCleanup(self.temporary.cleanup)
        # 本组仅验证校验/发布流程；显式替换pins和流，绝不执行这些假字节。
        self.payload = b"controlled-test-stream-not-an-OPA-executable"
        self.asset = self.module.ReleaseAsset("opa_linux_amd64_static", len(self.payload), hashlib.sha256(self.payload).hexdigest())

    def streams(self, checksum=None, payload=None):
        checksum = checksum if checksum is not None else self.asset.sha256.encode() + b"\n"
        payload = self.payload if payload is None else payload
        return patch.object(self.module, "_open_https", side_effect=[io.BytesIO(checksum), io.BytesIO(payload)])

    def prepare(self):
        with patch.object(self.module, "_asset_for_platform", return_value=self.asset):
            return self.module.prepare(self.output)

    def test_published_pins_are_exact_official_assets(self):
        self.assertEqual(self.module.VERSION, "1.21.2")
        assets = {asset.name: (asset.size, asset.sha256) for asset in self.module.ASSETS.values()}
        self.assertEqual(assets, {
            "opa_windows_amd64.exe": (64622592, "0089fa7ad8cdea7ff9e56ff86fa9202462afb63ef74506e2b547a24b4a987ccb"),
            "opa_linux_amd64_static": (64181524, "830ab11f61ffec760b5e19db0d7ad02bf037e13f5ef4d665faeaf17632177527"),
        })

    def test_real_cli_refuses_local_call_before_any_download(self):
        env = dict(os.environ, GITHUB_ACTIONS="false")
        result = subprocess.run([sys.executable, "-B", str(SCRIPT), "--output", str(self.output)], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("GitHub Actions", result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertFalse(self.output.exists())

    def test_each_ci_precondition_is_required(self):
        for key, value in [("GITHUB_ACTIONS", "false"), ("CI", "false"), ("GITHUB_RUN_ID", "not-a-run"), ("RUNNER_TEMP", "relative")]:
            with self.subTest(key=key), patch.dict(os.environ, {key: value}), patch.object(self.module, "_open_https") as network:
                with self.assertRaises(ValueError):
                    self.prepare()
                network.assert_not_called()

    def test_unsupported_platform_and_architecture_are_rejected(self):
        for system, machine in [("Darwin", "x86_64"), ("Linux", "aarch64"), ("Windows", "ARM64")]:
            with self.subTest(system=system, machine=machine), patch.object(self.module.platform, "system", return_value=system), patch.object(self.module.platform, "machine", return_value=machine):
                with self.assertRaises(ValueError):
                    self.module._asset_for_platform()

    def test_output_must_be_strictly_inside_runner_temp(self):
        for output in [Path("relative"), self.root, self.root / ".." / "escaped-opa", self.root.parent / "escaped-opa"]:
            with self.subTest(output=output), patch.object(self.module, "_open_https") as network:
                with self.assertRaises(ValueError):
                    self.module.prepare(output)
                network.assert_not_called()

    def test_linked_parent_is_rejected(self):
        target = self.root / "ordinary"
        target.mkdir()
        link = self.root / "linked"
        try:
            link.symlink_to(target, target_is_directory=True)
        except OSError:
            # 无创建链接权限的Windows仍覆盖reparse识别，不冒称真实OS链接已创建。
            from types import SimpleNamespace
            self.assertTrue(self.module._linked(SimpleNamespace(st_mode=0, st_file_attributes=0x400)))
            return
        with self.assertRaises(ValueError):
            self.module.prepare(link / "opa")

    def test_existing_mismatch_is_never_overwritten(self):
        for payload in [b"wrong size", b"x" * len(self.payload)]:
            self.output.write_bytes(payload)
            with patch.object(self.module, "_open_https") as network:
                with self.assertRaises(ValueError):
                    self.prepare()
                network.assert_not_called()
            self.assertEqual(self.output.read_bytes(), payload)

    def test_existing_directory_is_not_an_executable(self):
        self.output.mkdir()
        with self.assertRaises(ValueError):
            self.prepare()
        self.assertTrue(self.output.is_dir())

    def test_bad_official_checksum_prevents_binary_download_and_execute(self):
        with self.streams(b"0" * 64 + b"\n") as network, patch.object(self.module, "_make_executable") as executable:
            with self.assertRaises(ValueError):
                self.prepare()
            self.assertEqual(network.call_count, 1)
            executable.assert_not_called()
        self.assertFalse(self.output.exists())

    def test_checksum_wrong_filename_or_extra_content_is_rejected(self):
        for value in [self.asset.sha256.encode() + b"  another-asset\n", self.asset.sha256.encode() + b"\nextra", b"x" * 2048]:
            with self.subTest(value=value[:12]), self.streams(value):
                with self.assertRaises(ValueError):
                    self.prepare()
        self.assertFalse(self.output.exists())

    def test_bad_binary_size_or_digest_never_publishes(self):
        for payload in [self.payload[:-1], self.payload + b"x", b"x" * len(self.payload)]:
            with self.subTest(size=len(payload)), self.streams(payload=payload), patch.object(self.module, "_make_executable") as executable:
                with self.assertRaises(ValueError):
                    self.prepare()
                executable.assert_not_called()
            self.assertFalse(self.output.exists())
            self.assertEqual(list(self.directory.iterdir()), [])

    def test_controlled_success_checks_before_execute_and_returns_fixed_pin(self):
        def execute(path):
            self.assertEqual(path.read_bytes(), self.payload)
        original_path = os.environ.get("PATH")
        with self.streams() as network, patch.object(self.module, "_make_executable", side_effect=execute) as executable:
            metadata = self.prepare()
            self.assertTrue(network.call_args_list[0].args[0].endswith(self.asset.name + ".sha256"))
            self.assertTrue(network.call_args_list[1].args[0].endswith(self.asset.name))
            executable.assert_called_once()
        self.assertEqual(metadata, {"path": str(self.output), "version": "1.21.2", "asset": self.asset.name, "size": len(self.payload), "sha256": "sha256:" + self.asset.sha256})
        self.assertEqual(os.environ.get("PATH"), original_path)
        self.assertEqual(list(self.directory.iterdir()), [self.output])

    def test_existing_match_rechecks_official_checksum_without_redownloading(self):
        self.output.write_bytes(self.payload)
        with self.streams() as network, patch.object(self.module, "_make_executable"):
            self.prepare()
            self.assertEqual(network.call_count, 1)
        self.assertEqual(self.output.read_bytes(), self.payload)

    def test_destination_race_never_overwrites_another_file(self):
        original_link = os.link
        def race(source, destination, **kwargs):
            self.output.write_bytes(b"other owner")
            return original_link(source, destination, **kwargs)
        with self.streams(), patch.object(self.module.os, "link", side_effect=race), patch.object(self.module, "_make_executable") as executable:
            with self.assertRaises(FileExistsError):
                self.prepare()
            executable.assert_not_called()
        self.assertEqual(self.output.read_bytes(), b"other owner")

    def test_https_redirects_reject_untrusted_or_non_https_destinations(self):
        for url in ["http://github.com/opa", "https://evil.invalid/opa", "file:///tmp/opa"]:
            with self.subTest(url=url), self.assertRaises(ValueError):
                self.module._trusted_url(url)


if __name__ == "__main__":
    unittest.main()
