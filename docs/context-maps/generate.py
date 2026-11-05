"""Validate CML models and render SVG documentation with Context Mapper CLI."""

import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request
import xml.etree.ElementTree as ET


VERSION = "6.12.0"
SHA256 = "1a3e2a6a48b428c32831c8f43e6152f45e2538836bad8852cea7e0ca6c04f6b6"
URL = (
    "https://repo.maven.apache.org/maven2/org/contextmapper/context-mapper-cli/"
    f"{VERSION}/context-mapper-cli-{VERSION}.tar"
)
ROOT = Path(__file__).resolve().parent


def run_cli(java, classpath, *args):
    result = subprocess.run(
        [java, "-cp", classpath, "org.contextmapper.cli.ContextMapperCLI", *args],
        capture_output=True,
        text=True,
        timeout=120,
        check=True,
    )
    print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="")
    return result.stdout


def generate():
    java_home = os.environ.get("JAVA_HOME")
    java = str(Path(java_home) / "bin" / "java") if java_home else shutil.which("java")
    if not java or not shutil.which("dot"):
        raise RuntimeError("Install Java 17+ and Graphviz; put dot on PATH and set JAVA_HOME.")
    models = sorted(ROOT.glob("*.cml"))
    if not models:
        raise RuntimeError("No CML models found.")

    # Keep tools and intermediate PNG/DOT outputs outside the checkout.
    with tempfile.TemporaryDirectory(prefix="wg-context-maps-") as temporary:
        work = Path(temporary)
        archive_path = work / "cli.tar"
        with urllib.request.urlopen(URL, timeout=60) as response:
            archive_path.write_bytes(response.read())
        if hashlib.sha256(archive_path.read_bytes()).hexdigest() != SHA256:
            raise RuntimeError("Context Mapper CLI archive checksum does not match.")
        with tarfile.open(archive_path) as archive:
            archive.extractall(work / "tools", filter="data")
        classpath = str(work / "tools" / f"context-mapper-cli-{VERSION}" / "lib" / "*")
        output = work / "generated"
        output.mkdir()
        rendered = {}
        for model in models:
            validation = run_cli(java, classpath, "validate", "-i", str(model))
            # CLI 6.12.0 returns exit 0 for some reported errors.
            if "has been validated without errors." not in validation:
                raise RuntimeError(f"CML validation failed: {model.name}")
            run_cli(java, classpath, "generate", "-i", str(model),
                    "-g", "context-map", "-o", str(output))
            graph = output / f"{model.stem}_ContextMap.gv"
            # Context Mapper randomizes egg orientations on every invocation.
            # Fix those angles and remove its temporary image search path so
            # unchanged models do not cause an endless stream of bot commits.
            source = graph.read_text()
            source = re.sub(r'"orientation"="[^"]*"', '"orientation"="0"', source)
            source = re.sub(r'^graph \["imagepath"="[^"]*"\]\n', '', source, flags=re.M)
            graph.write_text(source)
            svg = output / f"{model.stem}_ContextMap.svg"
            subprocess.run(["dot", "-Tsvg", str(graph), "-o", str(svg)],
                           timeout=120, check=True)
            if not svg.is_file() or ET.parse(svg).getroot().tag != "{http://www.w3.org/2000/svg}svg":
                raise RuntimeError(f"No valid SVG generated for {model.name}")
            rendered[f"{model.stem}.svg"] = svg.read_bytes()

        # Publish only after every model validates and renders successfully.
        destination = ROOT / "generated"
        destination.mkdir(exist_ok=True)
        for name, content in rendered.items():
            (destination / name).write_bytes(content)
        for old in destination.glob("*.svg"):
            if old.name not in rendered:
                old.unlink()
        print(f"Generated {len(rendered)} context maps in {destination}")


if __name__ == "__main__":
    generate()
