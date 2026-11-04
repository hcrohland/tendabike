# Coding Standards

## Comments

Comments describe what the code **does**, not what **changed**. The commit message is where the change is explained; the comment is where the code is explained.

- No issue references in comments (`issue #717`, `Closes #N`, etc.)
- No change-language: "no longer", "previously", "now carries", "was removed", "moved to"
- No restating the diff: if the code says `message: None`, the comment doesn't need to say "the message is no longer stored here"

## User-facing strings

Reuse an existing `action_*` message (`action_delete`, `action_close`, `action_cancel`, …) for a UI action label rather than coining a per-feature one. A new message is for genuinely new phrasing, not a rewording of an action the app already expresses.
