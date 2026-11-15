#!/usr/bin/env python3
"""Enforce ADR-0015: the facade module is the sole importer of the callee.

For each (caller crate A, callee crate B) pair, at most one non-test module
in A may import from B — a `use tb_B::` statement, or a `tb_B::` path named
anywhere in the module (code or doc comment). The facade module is simply
whichever module imports from B; the rule needs no (A, B, module) list.

Test modules are exempt (`*_tests.rs`, `tests/`, `test_support/`), as are
test-gated bits of otherwise non-test files (`#[cfg(test)]` imports and
`#[cfg(test)] mod` blocks). A grandfathered exemption list covers the
crossings not yet conformed to the facade-module pattern — every edge except
the two conformed caller-side edges (`tb_exec`/`tb_provider` -> `tb_domain`);
new crossings must conform.

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

# A bare `#[cfg(test)]` attribute line: the next line is test-gated.
CFG_TEST_RE = re.compile(r"^\s*#\[\s*cfg\(\s*test\s*\)\s*\]\s*$")
# A `mod` declaration (the test-gated module case).
MOD_RE = re.compile(r"^\s*mod\s+\w+")


def is_exempt(relpath: str) -> bool:
    """Test locations a module may import from any callee.

    `*_tests.rs` files, the `tests/` suites, and the `test_support`
    doubles (a `test_support/` directory or the flat `test_support.rs`
    module) are test code — they import directly on purpose.
    """
    parts = relpath.split(os.sep)
    if relpath.endswith("_tests.rs"):
        return True
    if "tests" in parts:
        return True
    if "test_support" in parts or relpath.endswith("test_support.rs"):
        return True
    return False


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
    """Record every (caller -> callee) import in a non-exempt file.

    Test-gated code is skipped: a `#[cfg(test)]` attribute on the line
    before a `use`, and the body of a `#[cfg(test)] mod` block (tracked by
    brace depth).
    """
    with open(path) as f:
        lines = f.readlines()

    in_test_mod = 0  # brace depth inside a #[cfg(test)] mod block (0 = out)
    pending_cfg_test = False
    for lineno, line in enumerate(lines, 1):
        if in_test_mod:
            in_test_mod += line.count("{") - line.count("}")
            if in_test_mod <= 0:
                in_test_mod = 0
            continue
        if CFG_TEST_RE.match(line):
            pending_cfg_test = True
            continue
        if pending_cfg_test:
            pending_cfg_test = False
            if MOD_RE.match(line):
                in_test_mod = 1 + line.count("{") - line.count("}")
                if in_test_mod <= 0:
                    in_test_mod = 0
                continue
            if line.strip().startswith("use "):
                continue  # a test-gated import: not a crossing
            # otherwise the attribute gated something else on this line:
            # fall through and scan it
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
                if is_exempt(rel):
                    continue
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
