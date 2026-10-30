#!/usr/bin/env python3
"""Verify the Dockerfile's `build-deps` stubs cover every backend crate's targets.

The `build-deps` stage compiles the full dependency graph from the workspace
manifests alone, stubbing out all real source. Every `[lib]` needs a stub
`src/lib.rs`, every `[[bin]]` a stub at its `path`, and a plain binary crate a
stub `src/main.rs`. A `[[bin]]` (or `[lib]`) with no matching stub breaks
`cargo build` in that stage — and only the slow, remote `docker` CI job catches
it. This check runs locally at commit time so the gap is caught before push.

Fails (exit 1) listing any stub path the Dockerfile does not create.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOCKERFILE = os.path.join(ROOT, "Dockerfile")


def required_stubs(manifest: str, crate: str):
    """Return the repo-relative stub paths the Dockerfile must create for a crate."""
    with open(manifest) as f:
        text = f.read()

    has_lib = re.search(r"(?m)^\s*\[lib\]\s*$", text) is not None

    bins = []
    for block in re.findall(r"(?ms)\[\[bin\]\](.*?)(?=\n\[[^\[]|\Z)", text):
        m = re.search(r'path\s*=\s*"([^"]+)"', block)
        if m:
            bins.append(m.group(1))
        else:  # no explicit path -> Cargo default src/bin/<name>.rs
            nm = re.search(r'name\s*=\s*"([^"]+)"', block)
            if nm:
                bins.append("src/bin/" + nm.group(1).replace("-", "_") + ".rs")

    stubs = []
    if has_lib:
        stubs.append(f"backend/{crate}/src/lib.rs")
    stubs += [f"backend/{crate}/{b}" for b in bins]
    if not has_lib and not bins:  # default binary crate
        stubs.append(f"backend/{crate}/src/main.rs")
    return stubs


def main():
    with open(DOCKERFILE) as f:
        docker = f.read()

    missing = []
    for manifest in sorted(glob.glob(os.path.join(ROOT, "backend", "*", "Cargo.toml"))):
        crate = os.path.basename(os.path.dirname(manifest))
        for stub in required_stubs(manifest, crate):
            if stub not in docker:
                missing.append(stub)

    if missing:
        print("Dockerfile `build-deps` stage is missing stubs for:", file=sys.stderr)
        for p in missing:
            print(f"  - {p}", file=sys.stderr)
        print("Add each path to the build-deps stage in Dockerfile.", file=sys.stderr)
        sys.exit(1)
    print("Dockerfile build-deps stubs OK")


if __name__ == "__main__":
    main()
