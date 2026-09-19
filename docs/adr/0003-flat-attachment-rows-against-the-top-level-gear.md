# Flat attachment rows against the top-level gear

The attachment entity stores one flat row per attached part: `gear` is the top-level part of the assembly (the part itself while loose) and `hook` is the type of the part it is mounted on directly; the tree of mounts is derived from the rows, not stored. This was a deliberate choice to keep the entity free of redundant data while making activity registration a single query: activities record the gear, and every row of a gear's assembly has `attachment.gear == gear.id`.
