# Coding Standards

## Props

Prop defaults are set in the `$props()` destructuring, not as `?? fallback`s at the use site. Declare `required = false`, then pass `{required}` — not `required={required ?? false}`.

## Comments

Comments describe what the code **does**, not what **changed**. The commit message is where the change is explained; the comment is where the code is explained.

- No issue references in comments (`issue #717`, `Closes #N`, etc.) — spec references (`spec #795`, `spec #795 §2.1`) carry design rationale and are allowed
- No change-language: "no longer", "previously", "now carries", "was removed", "moved to"
- No restating the diff: if the code says `message: None`, the comment doesn't need to say "the message is no longer stored here"

## User-facing strings

Reuse an existing message that already expresses the same text rather than adding a duplicate. The shared `action_*` labels (`action_delete`, `action_close`, …) cover most buttons and are the first place to look; a new message is for genuinely new phrasing, not a rewording of a string the app already has.

## Test guides

Each crate's test guide (`backend/docs/tests/<crate>.md`) is the navigation surface for its test infrastructure. When a test double, fixture, or seam is added, renamed, or removed, update the guide in the same commit — a guide pointing at a double that no longer exists is a broken pointer.

## Public surface

Every item has the narrowest visibility that compiles: `pub` only if a different crate names it, `pub(crate)` only if a different module names it, `private` otherwise. The rule reaches fields, not just functions and types. Enum variants are all-or-nothing (Rust has no per-variant visibility), so an enum stays `pub` if any variant is named externally.

The test: drop the modifier. If it still compiles, the item was over-exposed. A public item no consumer names is case-specific weight exported past the seam (see [Generality](#generality)).

## Generality

Flag designs that are more specific than their weakest consumer requires. The seam should be as general as the least-capable caller needs:

- A trait bound that only the concrete impl needs belongs on the impl, not the trait (consumers' mocks shouldn't carry it).
- Identical setup/teardown across multiple test modules is one helper, not N copies.
- A mock that implements traits the code under test never calls is over-broad; narrow it to what's exercised.

The test: could a second consumer (a new provider, a new test module) use this shape unchanged? If not, the shape is carrying case-specific weight that belongs elsewhere.
