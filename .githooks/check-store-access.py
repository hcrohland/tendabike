#!/usr/bin/env python3
"""Enforce ADR-0010: the entity owns its store.

A store method (`<entity>_<op>`, defined in the store traits in
`backend/domain/src/traits/` and `backend/strava/src/traits.rs`) may be called
only from its owning entity's code. A cross-entity need goes through the
callee's entity method, never its store. This check collects every store
method name and greps the backend for calls to it, failing if a call appears
outside the owning entity's file (plus the exempted locations: the sqlx store
impls, the `MockStore`/`MockStravaConn` doubles, the test and seam suites, and
the `build_snapshot.rs` seed tool).

A call is a store-variable call (`store.method(`) or a store-trait
qualification (`PartStore::method(`). Entity qualifications (`Part::method(`)
are entity methods, not store calls, and are not matched.

Fails (exit 1) listing every offending call.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BACKEND = os.path.join(ROOT, "backend")

# Store trait names: used to detect fully-qualified `Trait::method(` calls.
STORE_TRAITS = (
    "PartStore",
    "UserStore",
    "ShopStore",
    "ActivityStore",
    "AttachmentStore",
    "PartNoteStore",
    "UsageStore",
    "ServiceStore",
    "ServicePlanStore",
    "NotificationStore",
    "StravaDataStore",
)
STORE_TRAITS_ALT = "|".join(STORE_TRAITS)


def is_exempt(relpath: str) -> bool:
    """Locations a store method may be called from, for any entity.

    The sqlx store files *are* the store (they implement it); the
    `test_support` doubles are the store's test doubles; the test and seam
    suites drive the store on purpose; `build_snapshot.rs` seeds the
    prepopulated snapshot.
    """
    parts = relpath.split(os.sep)
    if relpath.endswith("_tests.rs"):
        return True
    if "tests" in parts:  # backend/*/tests/** (seam + integration suites)
        return True
    if "test_support" in parts:  # MockStore, MockStravaConn
        return True
    if "sqlx" in parts and "store" in parts:  # backend/sqlx/src/store/**
        return True
    if relpath.endswith("stravastore.rs"):  # StravaDataStore impl
        return True
    if relpath.endswith("build_snapshot.rs"):  # snapshot seed tool
        return True
    return False


def trait_sources():
    """Yield (trait_file, [owning_source_files]) for each store trait.

    A domain sub-trait `domain/src/traits/<x>.rs` is owned by
    `entities/<x>.rs` and everything under `entities/<x>/` (so the shop
    trait covers `shop/subscription.rs`). `StravaDataStore`
    (`strava/src/traits.rs`) is owned by `strava`'s `user.rs` and `event.rs`.
    """
    for tf in sorted(glob.glob(os.path.join(BACKEND, "domain", "src", "traits", "*.rs"))):
        x = os.path.basename(tf)[:-3]
        own = [os.path.join(BACKEND, "domain", "src", "entities", x + ".rs")]
        own += glob.glob(
            os.path.join(BACKEND, "domain", "src", "entities", x, "**", "*.rs"),
            recursive=True,
        )
        yield tf, own
    strava_tf = os.path.join(BACKEND, "strava", "src", "traits.rs")
    if os.path.exists(strava_tf):
        yield strava_tf, [
            os.path.join(BACKEND, "strava", "src", "user.rs"),
            os.path.join(BACKEND, "strava", "src", "event.rs"),
        ]


def method_names(trait_file: str):
    """The `async fn <name>(` method names declared in a trait file."""
    with open(trait_file) as f:
        return re.findall(r"\basync\s+fn\s+(\w+)\s*\(", f.read())


def main():
    # name -> set of owning source files (repo-relative), plus a compiled
    # (dot-call, trait-qualified-call) pattern pair.
    owners = {}
    patterns = {}
    for tf, own in trait_sources():
        own_rel = {os.path.relpath(o, ROOT) for o in own if os.path.exists(o)}
        for name in method_names(tf):
            owners.setdefault(name, set()).update(own_rel)
            patterns[name] = (
                re.compile(r"\." + name + r"\s*\("),
                re.compile(r"\b(" + STORE_TRAITS_ALT + r")\s*::" + name + r"\s*\("),
            )

    violations = []
    for rs in sorted(glob.glob(os.path.join(BACKEND, "**", "*.rs"), recursive=True)):
        rel = os.path.relpath(rs, ROOT)
        if is_exempt(rel):
            continue
        with open(rs) as f:
            for lineno, line in enumerate(f, 1):
                for name, (dot_re, qual_re) in patterns.items():
                    if rel in owners[name]:
                        continue
                    if dot_re.search(line) or qual_re.search(line):
                        violations.append((rel, lineno, name, line.strip()))

    if violations:
        print("Store-access violations (ADR-0010: the entity owns its store):", file=sys.stderr)
        for rel, lineno, name, line in violations:
            print(f"  {rel}:{lineno}: `{name}` outside its owning entity", file=sys.stderr)
            print(f"      {line}", file=sys.stderr)
        print(
            "Route the call through the owning entity's method "
            "(or add the file to an exemption if it is the store itself, "
            "a double, a test, or the seed tool).",
            file=sys.stderr,
        )
        sys.exit(1)
    print("Store access OK (ADR-0010: the entity owns its store)")


if __name__ == "__main__":
    main()
