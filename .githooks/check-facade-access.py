#!/usr/bin/env python3
"""Enforce ADR-0015: the facade module is the sole importer of the callee.

For each (caller crate A, callee crate B) pair, at most one module in A may
import from B — a `use tb_B::` statement, or a `tb_B::` path named anywhere
in the module (code or doc comment). The facade module is simply whichever
module imports from B; the rule needs no (A, B, module) list.

The rule is universal — test modules included: the facade re-exports the
test-only domain types under the same cfg gate as the test code that names
them, so test code funnels through the facade too (no test exemption). A
grandfathered exemption list covers the crossings not yet conformed to the
facade-module pattern — every edge except the two conformed caller-side
edges (`tb_exec`/`tb_provider` -> `tb_domain`); new crossings must conform.

Store-impl edges (`tb_sqlx` implementing the `Store`/`StravaDataStore`/
`IcuDataStore` traits) run the other direction and stay under
`.githooks/check-store-access.py` (ADR-0012) — not covered here.

Fails (exit 1) listing every offending import.
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BACKEND = os.path.join(ROOT, "backend")

# Grandfathered crossings: every (caller, callee) edge not yet conformed to
# the facade-module pattern (ADR-0015's migration policy). The conformed
# edges — `tb_exec` -> `tb_domain` (exec::facade) and `tb_provider` ->
# `tb_domain` (provider::facade) — are absent on purpose: they are held to
# the sole-importer rule.
EXEMPT_EDGES = {
    ("tendabike", "tb_axum"),
    ("tendabike", "tb_exec"),
    ("tendabike", "tb_provider"),
    ("tendabike", "tb_sqlx"),
    ("tb_axum", "tb_domain"),
    ("tb_axum", "tb_exec"),
    ("tb_axum", "tb_provider"),
    ("tb_exec", "tb_provider"),
    ("tb_sqlx", "tb_domain"),
    ("tb_sqlx", "tb_exec"),
    ("tb_sqlx", "tb_provider"),
}

def crate_names():
    """The workspace's crate names, keyed by their backend/ directory."""
    crates = {}
    for entry in sorted(os.listdir(BACKEND)):
        manifest = os.path.join(BACKEND, entry, "Cargo.toml")
        if os.path.isfile(manifest):
            with open(manifest) as f:
                m = re.search(r'^name\s*=\s*"([^"]+)"', f.read(), re.M)
            if m:
                crates[entry] = m.group(1)
    return crates


def scan_file(path, rel, caller, crates, crossings):
    """Record every (caller -> callee) import in a file.

    Every line is scanned — test modules included (the rule is universal;
    test code funnels through the facade, ADR-0015).
    """
    with open(path) as f:
        lines = f.readlines()

    for lineno, line in enumerate(lines, 1):
        for callee in crates.values():
            if callee == caller:
                continue
            if re.search(r"\b" + re.escape(callee) + r"::", line):
                crossings.setdefault((caller, callee), []).append((rel, lineno, line.strip()))


def main():
    crates = crate_names()
    crossings = {}
    for entry, caller in crates.items():
        crate_dir = os.path.join(BACKEND, entry)
        for root, _, files in os.walk(crate_dir):
            for fn in sorted(files):
                if not fn.endswith(".rs"):
                    continue
                path = os.path.join(root, fn)
                rel = os.path.relpath(path, ROOT)
                scan_file(path, rel, caller, crates, crossings)

    violations = []
    for (caller, callee) in sorted(crossings):
        if (caller, callee) in EXEMPT_EDGES:
            continue
        imports = crossings[(caller, callee)]
        modules = {rel for rel, _, _ in imports}
        if len(modules) > 1:
            violations.append((caller, callee, imports))

    if violations:
        print(
            "Facade-access violations (ADR-0015: the facade module is the sole importer):",
            file=sys.stderr,
        )
        for caller, callee, imports in violations:
            modules = {rel for rel, _, _ in imports}
            print(
                f"  {caller} -> {callee}: {len(modules)} modules import directly (at most one may):",
                file=sys.stderr,
            )
            for rel, lineno, line in imports:
                print(f"    {rel}:{lineno}: {line}", file=sys.stderr)
        print(
            "Route the imports through the edge's facade module (re-export the "
            "needed types from it), or add the edge to the exemption list if "
            "it is grandfathered.",
            file=sys.stderr,
        )
        sys.exit(1)
    print("Facade access OK (ADR-0015: the facade module is the sole importer of the callee)")


if __name__ == "__main__":
    main()
