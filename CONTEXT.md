# Tendabike

A bike maintenance tracker that syncs with Strava: users track parts, schedule services, and log cycling activities.

## Language

### State

**Summary**:
The payload an operation or sync endpoint returns, carrying the entities it produced or touched. In a full read it carries the user's entire state; in responses to mutations or event draining it carries only the changed entities.
_Avoid_: snapshot, diff (each names only one of the two readings)

### Bike model

**type**:
The kind of thing a part is (e.g. tire, chain, wheel). Every type points by its main reference at the category it belongs to.

**category**:
A type which cannot be attached to another type — a top type such as bike, ski, or shoe.
_Avoid_: main (that is the name of the reference a type points at its category by)

**gear**:
A part whose type is a category — a bike, a ski. Other parts attach to gear.

**hook**:
A mount position that a type can attach to, expressed as the type of the part it attaches onto (a tire hooks onto a wheel). The word is overloaded in the codebase with the Strava event-drain endpoint; in domain discussion it means the mount position.

**attachment**:
A part being mounted on another part at a hook, over a span of time.
_Avoid_: file
