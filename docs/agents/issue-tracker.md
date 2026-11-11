# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations. `gh` infers the repo from the git remote; no `owner`/`repo` params needed.

## Conventions

- **Feature label**: one `feature:<slug>` label per feature.
- **Non-feature issues** (bugs, flaky tests, chores): apply the triage label only — no `feature:` label.
- **Labels are additive/removal**: `gh issue edit --add-label` / `--remove-label` modify the set; `--label` replaces it entirely.
- **Labels must exist before use**: create with `gh label create <name> --color <hex>` (bare hex like `0366d6`, not a UI color name).
- **Sub-issues and dependencies** use issue numbers (`#N`), not database ids.
- **Close an issue**: `gh issue close <n> --reason completed`.
- **Creating a PR**: write the body with the `pr` skill (there is no PR template in the repo).
- **Close from a PR**: a squash merge **collects each commit's `Closes #N` line** into the squashed commit, so each ticket closes via the `Closes #N` in its **own commit** (one issue per line — a line with several space-separated numbers closes only the first). The PR body carries `Closes` for the **umbrella spec only**. Take the collection as a given: just merge and GitHub assembles the squash commit (title + PR body + each commit's `Closes` line) — no verifying, no custom squash commit message.
- **Sync**: merge `main` into the feature branch (`git merge origin/main`); the `.githooks/pre-push` hook rejects force-pushes (non-fast-forward).

## Common operations

| Operation | Command |
|---|---|
| List open issues | `gh issue list --state open --json number,title,assignees,labels` |
| View an issue | `gh issue view <n> --json title,body,labels,assignees,state,comments` |
| Create an issue | `gh issue create --title "…" --body "…" --label "…"` |
| Create as sub-issue | `gh issue create --title "…" --body "…" --parent <n>` |
| Create blocked by | `gh issue create --title "…" --body "…" --blocked-by <n1>,<n2>` |
| Edit labels | `gh issue edit <n> --add-label "…" --remove-label "…"` |
| Assign | `gh issue edit <n> --add-assignee <login>` |
| Comment | `gh issue comment <n> --body "…"` |
| Close | `gh issue close <n> --reason completed` |
| Merge PR | `gh pr checks <n> --watch --interval 30 --fail-fast && gh pr merge <n> --squash` |
| Add sub-issue | `gh issue edit <parent> --add-sub-issue <child1>,<child2>` |
| Add blocked-by | `gh issue edit <n> --add-blocked-by <blocker1>,<blocker2>` |
| Add blocking | `gh issue edit <n> --add-blocking <n1>,<n2>` |
| Check blockers | `gh issue view <n> --json blockedBy` |

**Check gotcha:** `gh pr checks --watch` exits as soon as the checks that are *registered* pass — before the main CI jobs start, only CodeQL is listed, so it can report green early. Use it to block, then confirm a run exists for the head SHA before acting on the result: `gh run list --branch <head-branch> --limit 1`. Applies both when waiting between commits and before merging.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: try `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

`gh issue create`.

## When a skill says "fetch the relevant ticket"

`gh issue view <n> --json title,body,labels,assignees,state,comments`.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's native issue dependencies (UI-visible). Add with `gh issue edit <n> --add-blocked-by <blocker>`. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list open issues with `wayfinder:` labels (excluding `wayfinder:map`), filter to unassigned, then check `blockedBy` on the candidates. First in number order wins.
  ```bash
  gh issue list --state open --json number,title,assignees,labels,blockedBy \
    | jq '[.[] | select(.labels | any(.name | startswith("wayfinder:") and .name != "wayfinder:map")) | select((.assignees | length) == 0) | select((.blockedBy | length) == 0)]'
  ```
- **Claim**: `gh issue edit <n> --add-assignee <login>` — the session's first write.
- **Resolve**: `gh issue comment <n> --body "…"` → `gh issue close <n> --reason completed` → append to the map's Decisions-so-far (read its body with `gh issue view`, append the one-line gist + link, `gh issue edit <map> --body "…"`).
