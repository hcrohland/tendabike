#!/usr/bin/env python3
"""Flag issue references in comment lines of the staged diff.

CODING_STANDARDS.md ("Comments") forbids issue references in comments:
the commit message is where the change is explained, the comment is
where the code is explained. This check reads the staged additions and
fails when a comment line references an issue (`issue #N`, `ticket #N`,
`Closes #N`). Spec references (`spec #N`, `spec #N §X.Y`) carry design
rationale and are allowed.

Comment syntax per file type:
  .rs .ts .svelte  ->  //
  .py .sh .bash    ->  #

Fails (exit 1) listing the offending lines.
"""
import os
import re
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

ISSUE_REF = re.compile(r"(?:issue|ticket)\s+#\d+|closes\s+#\d+", re.IGNORECASE)


def main():
    diff = subprocess.run(
        ["git", "diff", "--cached", "--unified=0"],
        capture_output=True,
        text=True,
        cwd=ROOT,
    ).stdout

    offenders = []
    current_file = None
    current_prefix = None

    for line in diff.splitlines():
        if line.startswith("+++ "):
            path = line[4:].strip()
            if path.startswith("b/"):
                path = path[2:]
            current_file = path
            _, ext = os.path.splitext(path)
            current_prefix = COMMENT_PREFIXES.get(ext)
        elif line.startswith("+") and not line.startswith("+++"):
            if current_prefix is None:
                continue
            stripped = line[1:].lstrip()
            if stripped.startswith(current_prefix) and ISSUE_REF.search(stripped):
                offenders.append(f"  {current_file}: {stripped}")

    if not offenders:
        return

    print("Issue references in comments (CODING_STANDARDS.md, Comments):")
    for offender in offenders:
        print(offender)
    print()
    print(
        "Comments explain what the code does; the commit message explains "
        "what changed. Spec references (spec #N) are allowed."
    )
    sys.exit(1)


if __name__ == "__main__":
    main()
