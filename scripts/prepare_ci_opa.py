"""仅为 GitHub Actions 准备固定 OPA 原件；不安装到全局，也不修改 PATH。"""
import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import stat
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlsplit
from urllib.request import build_opener, HTTPRedirectHandler, Request


VERSION = "1.21.2"


@dataclass(frozen=True)
class ReleaseAsset:
    """经官方 release API 和独立 SHA256 核对的固定资产，不由当前下载内容自签。"""
    name: str
    size: int
    sha256: str


ASSETS = {
    ("Windows", "amd64"): ReleaseAsset("opa_windows_amd64.exe", 64622592,
        "0089fa7ad8cdea7ff9e56ff86fa9202462afb63ef74506e2b547a24b4a987ccb"),
    ("Linux", "amd64"): ReleaseAsset("opa_linux_amd64_static", 64181524,
        "830ab11f61ffec760b5e19db0d7ad02bf037e13f5ef4d665faeaf17632177527"),
}


def _asset_for_platform():
    machine = platform.machine().lower()
    if machine not in {"amd64", "x86_64"}:
        raise ValueError("only official Windows/Linux amd64 OPA CI assets are supported")
    try:
        return ASSETS[(platform.system(), "amd64")]
    except KeyError as error:
        raise ValueError("only official Windows/Linux amd64 OPA CI assets are supported") from error


def _linked(metadata):
    return stat.S_ISLNK(metadata.st_mode) or bool(getattr(metadata, "st_file_attributes", 0) & 0x400)


def _safe_existing(path):
    for part in [path, *path.parents]:
        metadata = part.lstat()
        if _linked(metadata):
            raise ValueError("linked/reparse CI paths are forbidden")
    return path.resolve(strict=True)


def _destination(output):
    if (os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("CI") != "true"
            or not re.fullmatch(r"[1-9][0-9]*", os.environ.get("GITHUB_RUN_ID", ""))):
        raise ValueError("OPA preparation is restricted to an explicit GitHub Actions run")
    raw_root = Path(os.environ.get("RUNNER_TEMP", ""))
    if not raw_root.is_absolute() or ".." in raw_root.parts:
        raise ValueError("GitHub Actions RUNNER_TEMP must be an absolute existing directory")
    root = _safe_existing(raw_root)
    if not root.is_dir() or not output.is_absolute() or ".." in output.parts:
        raise ValueError("OPA output must be strictly inside RUNNER_TEMP")
    parent = _safe_existing(output.parent)
    resolved = parent / output.name
    if resolved == root or not resolved.is_relative_to(root):
        raise ValueError("OPA output must be strictly inside RUNNER_TEMP")
    try:
        output.lstat()
    except FileNotFoundError:
        pass
    else:
        _safe_existing(output)
    return resolved


def _trusted_url(url):
    parsed = urlsplit(url)
    hosts = {"github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com",
             "github-releases.githubusercontent.com"}
    if (parsed.scheme != "https" or parsed.hostname not in hosts or parsed.port not in {None, 443}
            or parsed.username is not None or parsed.password is not None or parsed.fragment):
        raise ValueError("only official HTTPS GitHub release asset redirects are allowed")
    return url


class _OfficialRedirects(HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, new_url):
        _trusted_url(new_url)
        return super().redirect_request(request, response, code, message, headers, new_url)


def _open_https(url):
    _trusted_url(url)
    request = Request(url, headers={"User-Agent": "guardengine-ci-opa/1", "Accept-Encoding": "identity"})
    response = build_opener(_OfficialRedirects()).open(request, timeout=30)
    _trusted_url(response.geturl())
    return response


def _verify_file(path, asset):
    metadata = path.lstat()
    if (_linked(metadata) or not stat.S_ISREG(metadata.st_mode) or metadata.st_size != asset.size
            or getattr(metadata, "st_nlink", 1) != 1):
        raise ValueError("existing OPA must be an unlinked regular file with the exact official size")
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as stream:
        while True:
            block = stream.read(min(1024 * 1024, asset.size + 1 - size))
            if not block:
                break
            size += len(block)
            if size > asset.size:
                raise ValueError("existing OPA changed beyond its fixed size")
            digest.update(block)
    if size != asset.size or digest.hexdigest() != asset.sha256:
        raise ValueError("existing OPA differs from the fixed official SHA256; refusing overwrite")


def _make_executable(path):
    if os.name != "nt":
        path.chmod(0o700)
    observed = subprocess.run([str(path), "version"], check=True, capture_output=True, text=True, timeout=15)
    lines = observed.stdout.splitlines()
    expected_platform = "windows/amd64" if platform.system() == "Windows" else "linux/amd64"
    if (not lines or lines[0] != f"Version: {VERSION}" or "Rego Version: v1" not in lines
            or f"Platform: {expected_platform}" not in lines):
        raise ValueError("fixed OPA version/platform/Rego probe did not match")


def prepare(output):
    """在受限 CI 临时目录校验/发布原件；已存在文件只允许精确复用，不覆盖。"""
    output = _destination(Path(output))
    asset = _asset_for_platform()
    exists = output.exists()
    if exists:
        _verify_file(output, asset)
    url = f"https://github.com/open-policy-agent/opa/releases/download/v{VERSION}/{asset.name}"
    with _open_https(url + ".sha256") as stream:
        checksum = stream.read(1025)
    try:
        official = checksum.decode("ascii")
    except UnicodeDecodeError as error:
        raise ValueError("official checksum must be bounded ASCII") from error
    pattern = rf"{asset.sha256}(?:[ \t]+\*?{re.escape(asset.name)})?\r?\n?"
    if len(checksum) > 1024 or not re.fullmatch(pattern, official):
        raise ValueError("official checksum/asset name differs from the independently fixed release pin")
    if not exists:
        temporary = None
        try:
            descriptor, name = tempfile.mkstemp(prefix=".opa-download-", dir=output.parent)
            temporary = Path(name)
            digest, size = hashlib.sha256(), 0
            deadline = time.monotonic() + 180
            with os.fdopen(descriptor, "wb") as target, _open_https(url) as stream:
                while True:
                    if time.monotonic() >= deadline:
                        raise ValueError("OPA CI download exceeded its deadline")
                    block = stream.read(min(1024 * 1024, asset.size + 1 - size))
                    if not block:
                        break
                    size += len(block)
                    if size > asset.size:
                        raise ValueError("OPA binary exceeded fixed official size")
                    digest.update(block)
                    target.write(block)
                target.flush()
                os.fsync(target.fileno())
            if size != asset.size or digest.hexdigest() != asset.sha256:
                raise ValueError("OPA binary size/SHA256 differs from fixed official asset")
            _destination(output)
            # 原子 create-if-absent：不能使用 replace/rename 覆盖另一个并发所有者的文件。
            os.link(temporary, output)
            temporary.unlink()
            temporary = None
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
    _destination(output)
    _verify_file(output, asset)
    _make_executable(output)
    return {"path": str(output), "version": VERSION, "asset": asset.name, "size": asset.size,
            "sha256": "sha256:" + asset.sha256}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        metadata = prepare(args.output)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"OPA CI preparation rejected: {error}", file=sys.stderr)
        return 2
    print(json.dumps(metadata, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
