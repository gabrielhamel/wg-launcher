#!/usr/bin/env python3
"""Build an APT-owned CLI and a signed repository from immutable release assets."""
import argparse
import gzip
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib
import urllib.request


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--gnupg-home", required=True)
    args = parser.parse_args()
    source = args.source.resolve()
    output = args.output.resolve()
    package = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]
    version = package["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version) or args.tag.removeprefix("v") != version:
        raise ValueError("The tag must match a stable workspace version")
    apt = output / "apt"
    pool = apt / "pool/main/w/wg-launcher"
    pool.mkdir(parents=True, exist_ok=True)
    # Retain earlier packages so old indexes and explicit-version installs remain usable.
    pages = json.loads(subprocess.check_output([
        "gh", "api", f"repos/{args.repository}/releases", "--paginate", "--slurp"
    ]))
    for page in pages:
        for release in page:
            if release["draft"] or release["prerelease"]:
                continue
            for asset in release["assets"]:
                name = asset["name"]
                if re.fullmatch(r"wg-launcher_\d+\.\d+\.\d+_amd64\.deb", name):
                    urllib.request.urlretrieve(asset["browser_download_url"], pool / name)
    deb = pool / f"wg-launcher_{version}_amd64.deb"
    new_package = not deb.exists()
    if new_package:
        env = dict(os.environ, WG_LAUNCHER_PACKAGE_MANAGER="apt")
        run("cargo", "build", "-p", "wg-launcher-cli", "--bin", "wg-launcher", "--profile", "dist", "--locked", cwd=source, env=env)
        with tempfile.TemporaryDirectory(prefix="deb-", dir=output) as temporary:
            root = Path(temporary)
            (root / "DEBIAN").mkdir()
            (root / "usr/bin").mkdir(parents=True)
            (root / "usr/share/doc/wg-launcher").mkdir(parents=True)
            shutil.copy2(source / "target/dist/wg-launcher", root / "usr/bin/wg-launcher")
            shutil.copy2(source / "crates/wg-launcher-cli/LICENSE", root / "usr/share/doc/wg-launcher/copyright")
            size = (root / "usr/bin/wg-launcher").stat().st_size // 1024 + 1
            (root / "DEBIAN/control").write_text(
                f"Package: wg-launcher\nVersion: {version}\nArchitecture: amd64\n"
                "Maintainer: Gabriel Hamel <gabriel.hamel.pro@gmail.com>\n"
                "Section: games\nPriority: optional\n"
                "Depends: libc6 (>= 2.35), libgcc-s1\n"
                f"Installed-Size: {size}\nHomepage: {package['homepage']}\n"
                "Description: Command-line launcher for Wargaming game assets\n"
                " Discover, download, install, and update game assets.\n"
            )
            run("dpkg-deb", "--build", "--root-owner-group", str(root), str(deb))
    index = apt / "dists/stable/main/binary-amd64"
    index.mkdir(parents=True, exist_ok=True)
    packages = subprocess.check_output(["apt-ftparchive", "packages", "pool"], cwd=apt)
    (index / "Packages").write_bytes(packages)
    (index / "Packages.gz").write_bytes(gzip.compress(packages, mtime=0))
    release_path = apt / "dists/stable/Release"
    options = []
    for key, value in {"Origin": "wg-launcher", "Label": "wg-launcher", "Suite": "stable", "Codename": "stable", "Architectures": "amd64", "Components": "main"}.items():
        options.extend(["-o", f"APT::FTPArchive::Release::{key}={value}"])
    release_path.write_bytes(subprocess.check_output(["apt-ftparchive", *options, "release", "dists/stable"], cwd=apt))
    gpg = ["gpg", "--homedir", args.gnupg_home, "--batch", "--yes", "--digest-algo", "SHA256"]
    (apt / "wg-launcher.asc").write_bytes(subprocess.check_output([*gpg, "--armor", "--export"]))
    run(*gpg, "--armor", "--clearsign", "--output", str(release_path.with_name("InRelease")), str(release_path))
    run(*gpg, "--armor", "--detach-sign", "--output", str(release_path.with_name("Release.gpg")), str(release_path))
    run(*gpg, "--verify", str(release_path.with_name("InRelease")))
    (output / ".nojekyll").touch()
    (output / "index.html").write_text('<!doctype html><title>wg-launcher packages</title><a href="https://github.com/gabrielhamel/wg-launcher#install-the-cli">Installation instructions</a>')
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as stream:
            stream.write(f"new_package={str(new_package).lower()}\nasset_path={deb}\n")
    print(f"APT repository ready: {deb}")


if __name__ == "__main__":
    main()
