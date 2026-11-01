#!/usr/bin/env python3
"""Flag verbatim comment lines repeated 3+ times in the staged diff.

A rationale comment copy-pasted across call sites (the same lines at
four methods) is a smell the formatter and clippy cannot see: each line
is individually valid, only the repetition is the problem. This check
reads the staged additions and fails when the same comment line (after
stripping leading whitespace) appears 3+ times.

Comment syntax per file type:
  .rs .ts .svelte  ->  //
  .py .sh .bash    ->  #

Fails (exit 1) listing the repeated comment and its count.
"""
import collections
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

COMMENT_PREFIXES = {
    ".rs": "//",
    ".ts": "//",
    ".svelte": "//",
    ".py": "#",
    ".sh": "#",
    ".bash": "#",
}

THRESHOLD = 3


def main():
    diff = subprocess.run(
        ["git", "diff", "--cached", "--unified=0"],
        capture_output=True,
        text=True,
        cwd=ROOT,
    ).stdout

    counts = collections.Counter()
    current_prefix = None

    for line in diff.splitlines():
        if line.startswith("+++ "):
            path = line[4:].strip()
            if path.startswith("b/"):
                path = path[2:]
            _, ext = os.path.splitext(path)
            current_prefix = COMMENT_PREFIXES.get(ext)
        elif line.startswith("+") and not line.startswith("+++"):
            if current_prefix is None:
                continue
            stripped = line[1:].lstrip()
            if stripped.startswith(current_prefix):
                counts[stripped] += 1

    repeated = {text: count for text, count in counts.items() if count >= THRESHOLD}
    if not repeated:
        return

    print(f"Repeated comment lines in staged diff (>= {THRESHOLD} occurrences):")
    for text, count in sorted(repeated.items(), key=lambda x: -x[1]):
        print(f"  {count}x  {text}")
    print()
    print("Extract the shared rationale into one place and reference it.")
    sys.exit(1)


if __name__ == "__main__":
    main()
