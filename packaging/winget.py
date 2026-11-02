#!/usr/bin/env python3
"""Generate the three WinGet manifests from a verified cargo-dist Windows archive."""

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import urllib.parse
import urllib.request
import zipfile

IDENTIFIER = "GabrielHamel.WgLauncher"
SCHEMA = "1.12.0"
REPOSITORY = "https://github.com/gabrielhamel/wg-launcher"


def yaml_text(value, indent=0):
    """Emit the small JSON-compatible YAML subset accepted by WinGet."""
    padding = " " * indent
    if isinstance(value, dict):
        lines = []
        for key, item in value.items():
            if isinstance(item, (dict, list)):
                lines.append(f"{padding}{key}:\n{yaml_text(item, indent + 2)}")
            else:
                lines.append(f"{padding}{key}: {json.dumps(item, ensure_ascii=False)}")
        return "\n".join(lines)
    return "\n".join(
        f"{padding}-\n{yaml_text(item, indent + 2)}"
        if isinstance(item, dict)
        else f"{padding}- {json.dumps(item)}"
        for item in value
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--validate", action="store_true")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    version = args.tag.removeprefix("v")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("WinGet publishing requires a stable release tag")
    base = f"{REPOSITORY}/releases/download/{urllib.parse.quote(args.tag, safe='')}/"
    manifest = json.load(
        urllib.request.urlopen(base + "dist-manifest.json", timeout=30)
    )
    app = next(
        app for app in manifest["releases"] if app["app_name"] == "wg-launcher-cli"
    )
    if app["app_version"] != version or manifest["announcement_is_prerelease"]:
        raise ValueError("The release tag and manifest version do not match")
    name = "wg-launcher-cli-x86_64-pc-windows-msvc.zip"
    expected = manifest["artifacts"][name]["checksums"]["sha256"]
    archive = urllib.request.urlopen(base + name, timeout=60).read()
    if hashlib.sha256(archive).hexdigest() != expected:
        raise ValueError("Windows archive checksum mismatch")
    import io

    with zipfile.ZipFile(io.BytesIO(archive)) as zipped:
        binaries = [
            name
            for name in zipped.namelist()
            if PurePosixPath(name).name == "wg-launcher.exe"
        ]
        if len(binaries) != 1 or ".." in PurePosixPath(binaries[0]).parts:
            raise ValueError(
                "Expected one safe launcher executable in the Windows archive"
            )
        nested = binaries[0]
    common = {"PackageIdentifier": IDENTIFIER, "PackageVersion": version}
    installer = {
        **common,
        "InstallerType": "zip",
        "NestedInstallerType": "portable",
        "UpgradeBehavior": "uninstallPrevious",
        "Commands": ["wg-launcher"],
        "Installers": [
            {
                "Architecture": "x64",
                "InstallerUrl": base + name,
                "InstallerSha256": expected.upper(),
                "NestedInstallerFiles": [
                    {"RelativeFilePath": nested, "PortableCommandAlias": "wg-launcher"}
                ],
            }
        ],
        "ManifestType": "installer",
        "ManifestVersion": SCHEMA,
    }
    locale = {
        **common,
        "PackageLocale": "en-US",
        "Publisher": "Gabriel Hamel",
        "PublisherUrl": "https://github.com/gabrielhamel",
        "PublisherSupportUrl": REPOSITORY + "/issues",
        "PackageName": "wg-launcher",
        "PackageUrl": REPOSITORY,
        "License": "MIT",
        "LicenseUrl": REPOSITORY + "/blob/main/crates/wg-launcher-cli/LICENSE",
        "ShortDescription": "Command-line launcher for Wargaming game assets",
        "Description": "Discover, download, install, and update Wargaming game assets. Player login and launching the game are outside its scope.",
        "Tags": ["cli", "wargaming", "launcher"],
        "ReleaseNotesUrl": REPOSITORY
        + "/releases/tag/"
        + urllib.parse.quote(args.tag, safe=""),
        "ManifestType": "defaultLocale",
        "ManifestVersion": SCHEMA,
    }
    version_manifest = {
        **common,
        "DefaultLocale": "en-US",
        "ManifestType": "version",
        "ManifestVersion": SCHEMA,
    }
    directory = args.output / "manifests/g/GabrielHamel/WgLauncher" / version
    directory.mkdir(parents=True, exist_ok=True)
    for suffix, kind, data in [
        (".installer", "installer", installer),
        (".locale.en-US", "defaultLocale", locale),
        ("", "version", version_manifest),
    ]:
        schema = f"https://aka.ms/winget-manifest.{kind}.{SCHEMA}.schema.json"
        path = directory / f"{IDENTIFIER}{suffix}.yaml"
        path.write_text(
            f"# yaml-language-server: $schema={schema}\n\n" + yaml_text(data) + "\n",
            encoding="utf-8",
        )
        if args.validate:
            import jsonschema
            import yaml

            official = json.load(urllib.request.urlopen(schema, timeout=30))
            jsonschema.validate(
                yaml.safe_load(path.read_text(encoding="utf-8")), official
            )
    print(directory)


if __name__ == "__main__":
    main()
