# Coding Standards

## Comments

Comments describe what the code **does**, not what **changed**. The commit message is where the change is explained; the comment is where the code is explained.

- No issue references in comments (`issue #717`, `Closes #N`, etc.)
- No change-language: "no longer", "previously", "now carries", "was removed", "moved to"
- No restating the diff: if the code says `message: None`, the comment doesn't need to say "the message is no longer stored here"
