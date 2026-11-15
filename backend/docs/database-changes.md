# Database changes

Schema migrations are rare and high-stakes. The frequent case stays in
`AGENTS.md`: a query change → `cargo sqlx prepare` (refresh the offline cache so
it compiles); a new migration → `cargo sqlx migrate run`. This file is the
disclosure target for the collision case and the rules that govern it.

A migration's version is its filename prefix: the **date** in front, an
**ordering serial** behind (e.g. `20261115` + `151216`). `cargo sqlx migrate add`
always mints the newest version.

## The rules
- A migration is **frozen once applied** — change its behaviour with a new
  migration, not by editing the applied one. The only way to unfreeze it is
  `cargo sqlx migrate revert`, and that works **only while the migration is
  uncommitted** (not in git). Once committed, it's frozen for good.
- **Never edit the database by hand, and never reset it.** Every change goes
  through `cargo sqlx migrate` (`run`, `revert`, `add`). `migrate override` is
  off limits.

## Resolve a migration collision
A **collision** is two migrations that can't both apply under one version — a
same-version clash from divergent branches. On the DB it shows as `migrate run`
refusing a duplicate version, or (if a renumber happened without a revert) a
checksum mismatch: *"previously applied but has been modified."* The DB has
applied at most one of the two — **yours** or **main's**.

**Prevent it:** merge `main` before committing your migration. The clean fix
below is only available while your migration is uncommitted; catching the clash
early keeps you in that window.

The rule: **a migration cannot keep a colliding version.** You yield yours (never
main's) and re-express it under a new version — but only once it is
**un-applied**.

**Precondition:** your colliding migration is the **latest applied** and
**uncommitted**. `migrate revert` undoes only the latest migration (and needs
your down file), and a committed migration is frozen — so if yours is committed,
or if migrations were applied after yours, stop and escalate. Don't rewrite
shared history.

1. **Make your migration un-applied.**
   - Yours was applied → `cargo sqlx migrate revert`.
   - Main's was applied, or neither was → leave the DB as is.
2. **Re-express it under a new version.** `cargo sqlx migrate add
   <description>` mints the newest version (later than main's, by construction).
   Move your up and down SQL into the new migration, then remove the old
   (colliding) one.
3. **Run both.** `cargo sqlx migrate run` applies main's (its version) and yours
   (the new one), in order.

Done when `migrate info` shows both installed with no "different checksum" flag
and `cargo sqlx prepare --check` passes.
