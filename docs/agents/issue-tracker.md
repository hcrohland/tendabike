# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations. `gh` infers the repo from the git remote; no `owner`/`repo` params needed.

## Conventions

- **Feature label**: one `feature:<slug>` label per feature.
- **Non-feature issues** (bugs, flaky tests, chores): apply the triage label only — no `feature:` label.
- **Labels are additive/removal**: `gh issue edit --add-label` / `--remove-label` modify the set; `--label` replaces it entirely.
- **Sub-issue id**: the `gh api` sub-issue endpoint takes the child's numeric **database id** (the `id` field from `gh issue view --json id`), not its `#number`.
- **Close an issue**: `gh issue close <n> --reason completed`.
- **Creating a PR**: write the body with the `pr` skill (there is no PR template in the repo).
- **Close from a PR**: a squash merge collects each commit's `Closes #N` line, so each ticket closes via the `Closes #N` in its **own commit** (one issue per line — a line with several space-separated numbers closes only the first). The PR body carries `Closes` for the **umbrella spec only**; do not duplicate the per-ticket `Closes` lines into the PR body.
- **Sync**: merge `main` into the feature branch (`git merge origin/main`); the `.githooks/pre-push` hook rejects force-pushes (non-fast-forward).

## Common operations

| Operation | Command |
|---|---|
| List open issues | `gh issue list --state open --json number,title,assignees,labels` |
| View an issue | `gh issue view <n> --json title,body,labels,assignees,state,comments` |
| Create an issue | `gh issue create --title "…" --body "…" --label "…"` |
| Edit labels | `gh issue edit <n> --add-label "…" --remove-label "…"` |
| Assign | `gh issue edit <n> --add-assignee <login>` |
| Comment | `gh issue comment <n> --body "…"` |
| Close | `gh issue close <n> --reason completed` |
| Add sub-issue | `gh api repos/{owner}/{repo}/issues/{parent}/sub_issues -X POST -f sub_issue_id={child_db_id}` |
| Add dependency | `gh api repos/{owner}/{repo}/issue-dependencies -X POST -f source_issue_number={child} -f target_issue_number={blocker} -f type=blocked_by` |
| Check blockers | `gh api graphql -f query='{ repository(owner:"{owner}",name:"{repo}") { issue(number:{n}) { blockingIssues(first:10) { nodes { number state } } } } }'` |

`{owner}`/`{repo}` in `gh api` paths: infer from `git remote -v` (e.g. `hcrohland`/`tendabike`).

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
- **Blocking**: GitHub's native issue dependencies (UI-visible). Add with the `gh api` dependency endpoint above. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list open issues with `wayfinder:` labels (excluding `wayfinder:map`), filter to unassigned, then check `blockingIssues` on the candidates. First in number order wins.
  ```bash
  gh issue list --state open --json number,title,assignees,labels \
    | jq '[.[] | select(.labels | any(.name | startswith("wayfinder:") and .name != "wayfinder:map")) | select((.assignees | length) == 0)]'
  ```
  Then for each candidate, verify no open blockers via the GraphQL check above.
- **Claim**: `gh issue edit <n> --add-assignee <login>` — the session's first write.
- **Resolve**: `gh issue comment <n> --body "…"` → `gh issue close <n> --reason completed` → append to the map's Decisions-so-far (read its body with `gh issue view`, append the one-line gist + link, `gh issue edit <map> --body "…"`).
