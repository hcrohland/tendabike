# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the GitHub MCP server for all operations — the `mcp__github__*` tools, not the `gh` CLI. Every tool takes `owner`/`repo`; infer the pair from `git remote -v`.

## Conventions

- **Create an issue**: `issue_write` with `method: "create"` — `title`, `body` (multi-line bodies go in the string as-is).
- **Feature label**: one `feature:<slug>` label per feature.
- **Link as a sub-issue**: `issue_write` with `parent_issue_number` for a new issue; `sub_issue_write` (method `add`) for an existing one — it takes the child's numeric **database id** (the `id` field, not its `#number` or `node_id`), which full `search_issues` / `get_sub_issues` entries carry.
- **Read an issue**: `issue_read` with `method: "get"` for details (body, labels, state, assignees, hierarchy flags) or `get_comments` for comments.
- **List issues**: `list_issues` with `state` and `labels` filters; request only the `fields` you need — `body` is the largest per-result data.
- **Comment on an issue**: `add_issue_comment` (`issue_number`, `body`).
- **Apply / remove labels**: `issue_write` with `method: "update"` — its `labels` param *sets* the full label set, so removal is re-setting the list without that label (read the current labels first).
- **Close**: `issue_write` with `method: "update"`, `state: "closed"`, `state_reason: "completed"`; pair with `add_issue_comment` when a closing comment is wanted.
- **Close from a PR**: the PR collects `Closes` lines from its commits on merge; a single `Closes` closes only its first issue.
- **Sync**: keep a feature branch up to date by merging `main` into it (`git merge origin/main`); never force-push to the remote.
- **Merge**: `merge_pull_request` with `merge_method: "squash"` — PRs are squash-merged, and the squashed commit takes the PR title, so write the PR title as the commit title you want on `main`.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the MCP PR tools:

- **Read a PR**: `pull_request_read` — `get` for details, `get_comments` for comments, `get_diff` for the diff.
- **List external PRs for triage**: `search_pull_requests` with `fields` including `author_association`, then keep only `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, or `NONE` (drop `OWNER`/`MEMBER`/`COLLABORATOR`).
- **Comment / close**: `add_issue_comment` (pass the PR number as `issue_number`) / `update_pull_request` with `state: "closed"`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: try `pull_request_read` `get` and fall back to `issue_read` `get`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue with `issue_write`.

## When a skill says "fetch the relevant ticket"

`issue_read` with `method: "get"`, plus `get_comments`.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `issue_write` with `labels: ["wayfinder:map"]`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (see Conventions). Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's **native issue dependencies**, the canonical, UI-visible representation. Add an edge with `issue_dependency_write` (method `add`, type `blocked_by`) — both sides take issue **numbers**, no id lookup. The live gate is `issue_dependencies_summary.blocked_by` (open blockers only), surfaced on `issue_read get_sub_issues` entries and `search_issues` results; a ticket is unblocked when every blocker is closed. The `issue_dependency_*` tools are feature-flagged in the GitHub MCP server (`issue_dependencies`) — where they're absent from the toolset, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body.
- **Frontier query**: `issue_read` with `method: "get_sub_issues"` on the map (children in map order, each carrying `state`, `issue_dependencies_summary`, `assignees`); drop any that is closed, has an open blocker (`blocked_by > 0`, or an open issue in the `Blocked by` line), or has an assignee; first in map order wins.
- **Claim**: `issue_write` with `method: "update"`, `assignees: [<login>]` (your login from `get_me`) — the session's first write.
- **Resolve**: `add_issue_comment` with the answer, then close with `issue_write` (`state: "closed"`, `state_reason: "completed"`), then append to the map's Decisions-so-far (read its body, append, `issue_write` update): one-line gist + link to the closed ticket.
