#!/usr/bin/env python3
"""Enforce ADR-0014: tb_exec is provider-ignorant.

The executor crate (`backend/exec`) must not name a provider-specific
concept: the provider markers (`StravaSpec`, `IcuSpec`), the `Provider`
enum's concrete variants (`Provider::Strava`, `Provider::Icu`), or the
per-provider lane fields (`strava_lane`, `icu_lane`). The `Provider` enum
itself is allowed — it is the ADR-0014 interface, the typed form of the
domain concept — and so is the stable seam `tb_provider::provider_of`
(bounded on the `ProviderStore` supertrait): a third provider changes the
bound and the lookup's body in the provider layer only, leaving the
executor's call site untouched. A third provider must leave `tb_exec`
untouched: the provider knowledge lives in the provider layer (the unified
lane, the dispatch fns, the `provider_of` lookup), and the executor drives
the seam.

This check greps `backend/exec/src` for the provider-specific concepts and
`backend/exec/Cargo.toml` for the `strava`/`icu` feature flags, failing on
any match. It is strict — no exemptions: the ADR-0014 restoration (the
unified-lane, `ProviderService`-relocation, and residues tickets) has
landed, so the crate is clean and this is a pure guard against regression.

Fails (exit 1) listing every offending match.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXEC = os.path.join(ROOT, "backend", "exec")

# Provider-specific concepts that must not be named in tb_exec. The
# `Provider` enum itself and the `provider_of` stable seam are allowed; only
# the concrete, provider-specific concepts are banned. A third provider adds
# its marker, enum variant, and lane field here.
CONCEPTS = (
    "StravaSpec",
    "IcuSpec",
    "Provider::Strava",
    "Provider::Icu",
    "strava_lane",
    "icu_lane",
)
CONCEPT_RE = re.compile("|".join(re.escape(c) for c in CONCEPTS))

# A provider feature named in tb_exec's manifest: either a bare feature key
# (`strava = [...]` in the [features] section) or a quoted feature reference
# (`features = ["strava", ...]` on a dependency). Word boundaries keep this
# from matching a crate name like `tb_strava`.
FEATURE_RE = re.compile(r"\b(strava|icu)\b")


def main():
    violations = []

    # Grep backend/exec/src for the provider-specific concepts.
    for rs in sorted(glob.glob(os.path.join(EXEC, "src", "**", "*.rs"), recursive=True)):
        rel = os.path.relpath(rs, ROOT)
        with open(rs) as f:
            for lineno, line in enumerate(f, 1):
                if CONCEPT_RE.search(line):
                    violations.append((rel, lineno, line.strip()))

    # Grep backend/exec/Cargo.toml for the strava/icu feature flags.
    manifest = os.path.join(EXEC, "Cargo.toml")
    if os.path.exists(manifest):
        rel = os.path.relpath(manifest, ROOT)
        with open(manifest) as f:
            for lineno, line in enumerate(f, 1):
                if FEATURE_RE.search(line):
                    violations.append((rel, lineno, line.strip()))

    if violations:
        print(
            "Provider-concept violations (ADR-0014: tb_exec is provider-ignorant):",
            file=sys.stderr,
        )
        for rel, lineno, line in violations:
            print(f"  {rel}:{lineno}: {line}", file=sys.stderr)
        print(
            "Move the provider knowledge to the provider layer (the unified "
            "lane, the dispatch fns, the `provider_of` lookup); the executor "
            "drives the seam. The `Provider` enum and `provider_of` are the "
            "allowed stable seam.",
            file=sys.stderr,
        )
        sys.exit(1)
    print("Provider-concept access OK (ADR-0014: tb_exec is provider-ignorant)")


if __name__ == "__main__":
    main()
