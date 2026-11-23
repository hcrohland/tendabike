# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations. `gh` infers the repo from the git remote; no `owner`/`repo` params needed.

## Conventions

- **Feature label**: one `feature:<slug>` label per feature.
- **Non-feature issues** (bugs, flaky tests, chores): apply the triage label only — no `feature:` label.
- **Labels are additive/removal**: `gh issue edit --add-label` / `--remove-label` modify the set; `--label` replaces it entirely.
- **Labels must exist before use**: create with `gh label create <name> --color <hex>` (bare hex like `0366d6`, not a UI color name).
- **Sub-issues and dependencies** use issue numbers (`#N`), not database ids.
- **Creating a PR**: write the body with the `pr` skill (there is no PR template in the repo).
- **Close from a PR**: a squash merge **collects each commit's `Closes #N` line** into the squashed commit, so each ticket closes via the `Closes #N` in its **own commit** (one issue per line — a line with several space-separated numbers closes only the first). The PR body carries `Closes` for the **umbrella spec only**. Take the collection as a given: just merge and GitHub assembles the squash commit (title + PR body + each commit's `Closes` line) — no verifying, no custom squash commit message.
- **Sync**: merge `main` into the feature branch (`git merge origin/main`); the `.githooks/pre-push` hook rejects force-pushes (non-fast-forward).

## Common operations

| Operation | Command |
|---|---|
| List open issues | `gh issue list --state open --json number,title,assignees,labels` |
| View an issue | `gh issue view <n> --json title,body,labels,assignees,state,comments,subIssues` |
| Create an issue | `gh issue create --title "…" --body "…" --label "…"` |
| Create as sub-issue | `gh issue create --title "…" --body "…" --parent <n>` |
| Create blocked by | `gh issue create --title "…" --body "…" --blocked-by <n1>,<n2>` |
| Edit labels | `gh issue edit <n> --add-label "…" --remove-label "…"` |
| Assign | `gh issue edit <n> --add-assignee <login>` |
| Comment | `gh issue comment <n> --body "…"` |
| Close | `gh issue close <n> --reason completed` |
| Create PR | `gh pr create --head <branch> --title "…" --body "…"` |
| Check PR | sync to current `main`, then verify CI + mergeability — see **Check PR** below |
| Merge PR | **Check PR** (above), then `gh pr merge <n> --squash` |
| View PR inline review comments | `gh api repos/<owner>/<repo>/pulls/<n>/comments` |
| Reply to a review comment | `gh api repos/<owner>/<repo>/pulls/<n>/comments -F in_reply_to=<id> -f body="…"` |
| Add sub-issue | `gh issue edit <parent> --add-sub-issue <child1>,<child2>` |
| Add blocked-by | `gh issue edit <n> --add-blocked-by <blocker1>,<blocker2>` |
| Add blocking | `gh issue edit <n> --add-blocking <n1>,<n2>` |
| Check blockers | `gh issue view <n> --json blockedBy` |

**Check PR:** verify a PR on a current branch, in order:
1. **Sync** to current `main` (see the **Sync** convention) — `git fetch origin`; only if behind (`git rev-list --count HEAD..origin/main` > 0), `git merge origin/main`, resolve any conflict (see the `resolving-merge-conflicts` skill), then `git push`.
2. **CI:** `gh pr checks <n> --watch --interval 30 --fail-fast` — see **Check gotcha** (a run must exist for the head SHA).
3. **Mergeable:** `gh pr view <n> --json mergeable` reports `MERGEABLE`.

**Check gotcha:** `gh pr checks --watch` exits as soon as the checks that are *registered* pass — before the main CI jobs start, only CodeQL is listed, so it can report green early. Use it to block, then confirm a run exists for the head SHA before acting on the result: `gh run list --branch <head-branch> --limit 1`. Applies both when waiting between commits and before merging.

**Failure logs:** Get the run ID first (`gh run list --branch <b> --limit 1 --json databaseId --jq '.[0].databaseId'`), then `gh run view <id> --log-failed`. Don't chain the ID extraction into the view command.

**Review comments:** `gh pr view --json comments` returns only the review body, not the inline review comments. When replying, `-F in_reply_to=<id>` sends the id as a number — `-f` stringifies it and the API 422s.

**Body gotcha:** a markdown body with backticks (or other shell-special characters) passed inline to `--body` trips the shell's command-substitution scanner. Write the body to a temp file and pass `--body-file` instead.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: try `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

The **Create an issue** command above.

## When a skill says "fetch the relevant ticket"

The **View an issue** command above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's native issue dependencies (UI-visible) — the **Add blocked-by** command above. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list open issues with `wayfinder:` labels (excluding `wayfinder:map`), filter to unassigned, then check `blockedBy` on the candidates. First in number order wins.
  ```bash
  gh issue list --state open --json number,title,assignees,labels,blockedBy \
    | jq '[.[] | select(.labels | any(.name | startswith("wayfinder:") and . != "wayfinder:map")) | select((.assignees | length) == 0) | select(.blockedBy.nodes | all(.state == "CLOSED"))]'
  ```
- **Claim**: the **Assign** command above with `<login>` set to `@me` — the session's first write.
- **Resolve**: **Comment** → **Close** (commands above) → append to the map's Decisions-so-far in a single shell call: `gh issue view <map> --json body --jq .body > /tmp/map.md && sed -i '/^## Not yet specified/i\
- [<title>](<link>): <gist>' /tmp/map.md && gh issue edit <map> --body-file /tmp/map.md`.
