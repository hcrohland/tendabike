# The components

Your gear is only the beginning of what you maintain. The parts on it — the
tires, the chain, the brake pads — are the things that wear out and get
replaced, and TendaBike tracks each one: where it is, and for how long.

There are two places to look at parts. The **Parts** page in the top menu is
your shelf: every part you have entered, in one list, organised by kind.
Each piece of gear has an **Attached Parts** tab: what is on it right now,
laid out the way the parts sit on the bike.

## Where a part is — and for how long

A part is always either **on a bike** or **on another part** that is on a
bike: a chain is on the bike itself; a tire is on its wheel, and the wheel
is on the bike. When a part goes on, you say where, so the app knows which.

A part that carries others carries them as a group. A wheel with its tire,
a brake with its pads: move the part and the group moves with it, because
that is how they sit together on the real bike.

Every "on" has a beginning and an end. A part that is on right now shows
**since** and the day it went on. A part that has come off shows the whole
span — the day it went on and the day it came off — and each span keeps its
own riding numbers: the rides, hours, and kilometres of the time it was on
the bike. A part's counters count the riding it was actually on for.

## The Parts page

The **Parts** page has one box per kind of part — **Tire**, **Chain**,
**Brake pad**, **Cassette**, and so on; the kind is the small capitals label
at the top of the box. Inside, one card per part: its name, its brand and
model, the day you got it, and a row of small counters — rides, hours,
kilometres — for how much it has been used.

- A part that is on a bike right now shows **Attached to** and the name of
  what it is on. Those parts are collapsed under an **attached** toggle at
  the top of the box: the page is about your shelf, so the parts that are
  out riding are tucked away until you ask for them.
- A part you are done with is faded and shows **disposed** with the date.

Each card has a **⋮** menu:

- **Attach** — put the part on a bike, or on another part. For a part that
  is already on something, the item reads **Move** instead: the same
  dialog, and the part's old place simply ends on the new date.
- **Delete** — remove the part for good. It is only offered while the part
  has never been on anything; once it has a history, the item is
  **Dispose** instead.

## Add a part

You can create a part in two places.

**On the Parts page.** Each box has a **New** button in its corner. Click
it: a dialog opens, titled **New tire** (or whatever the kind is), with
three fields:

- **You call it** — the name.
- **and it is a** — the **Brand** and **Model**.
- **New tire day was** — the day you got it.

Fill them in and click **Create**. The part appears in the box — on your
shelf, not on a bike yet.

**Straight onto a bike.** Open the bike, go to the **Attached Parts** tab,
and click **add**. The dialog header reads **New [Choose part] of Gravel
Bike**: pick the kind from the list — **front tire**, **rear tire**,
**chain**, … — and the bike is already set. Fill in the same three fields
and click **Install**: the part is created and on the bike at the same
time.

## Put a part on

Open the part — from its card on the **Parts** page, or from any link to it
— and choose **Attach** from the **⋮** menu at the top right. The dialog
is titled after the part, like **Attach tire Front One Schwalbe One**, and
asks where it goes:

- **to** — only for kinds that can go in more than one place: which one,
  like **front wheel** or **rear wheel**.
- **of** — the bike it ends up on.
- **at** — the day it goes on.

Click **Attach**. The part's card now shows **Attached to** and what it is
on, and the bike's **Attached Parts** tab has it in the tree. If the place
is already taken, the whole assembly on it — the part and everything on the
part — comes off at the same time: the new part pushes it off.

## Move a part

A part that is on something shows **Move** where **Attach** would be — on
its card on the **Parts** page and in the tree. The dialog is the same:
choose the new **to**, **of**, and **at**, and click **Attach**. The part
is on the new place from the new date, and its old place ends on that same
date and stays in its history. If the part carries others — a wheel with
its tire — the group moves together and the tire comes along, no extra
step. And if the new place is already taken, the whole assembly on it is
pushed off at the same time.

## What a bike is carrying

Open a bike and the **Attached Parts** tab shows it as a tree: one card for
each kind of thing that can be on it.

- A card with a part on it shows the kind in small capitals, the part's
  name, **since** the day it went on, a badge for how close the part's
  service is to being due (see [Planned maintenance](service-plans.md)),
  and its counters.
- A card for an empty place shows just the kind and an **add** button:
  create a part for that place straight away.
- Cards nest the way the parts sit: the **Tire** card sits inside its
  wheel's card, the **Brake pad** card inside its brake's.
- A place that has had more than one part shows a **history** toggle: the
  earlier parts, each with its span and its own counters. From there you
  can **Attach** an old part again, or **Duplicate part** to create a new
  one from it.

For a part that is on, the card's **⋮** menu offers **Log Service**,
**Move**, and **Replace** with the kind's name — **Replace tire**, for
instance. **Replace** opens a dialog for the new part — **New tire for
Front Wheel** — prefilled with the part being replaced. For a part that
carries others, a **Keep all attached parts** switch decides what happens
to them: on, they are kept and end up loose on your shelf; off, they are
retired with the old part.

The part's own page keeps its full history in the **Attached to** panel
under the header: every bike — or part — it has been on, with the position
when there is one (**front**, **rear**) and the span, most recent first.
**history** shows the older ones; **Remove** in a row's menu drops that
span from the record.

## A new bike, quickly

Below the tree of a new bike, the app offers a shortcut for putting a whole
set of parts on at once. It lists the groups a bike is made of — **Wheels**,
**Tires**, **Drive train**, **Brakes**, and so on; if you have already added
some of them, an **Add more component groups** button brings up the rest.
Tick the groups you want, give each a brand and a model, and click **Set**.
A part is created for each group — named after the brand and model — and
put on the bike.

## When a part comes off

A part comes off in two ways, and the app keeps the difference.

- **Detach** — it comes off, but you keep it. Open the part, choose
  **Detach** from the **⋮** menu, and set the day. For a part that carries
  others — a wheel, say — **detach all attached parts** is on by default,
  so the parts on it stay on it: the whole group comes off the bike together
  as a loose unit, the tire still on the wheel. Untick it and the parts come
  off it too, ending up loose on your shelf. The dialog also offers
  **dispose tire when detached** — the kind's name, in plain words — if
  you are done with the part itself.
- **Dispose** — you are done with it. The menu shows **Dispose** for a part
  that is not on anything; set the day. **dispose all attached parts** is
  on by default, so the parts on it are retired with it; untick to keep
  them.

Behind the **Detach** switch is a quirk worth knowing: the app has no way
to put a loose tire onto a wheel by hand — you attach parts to a bike, not
to other parts. A group stays a group because detaching the part keeps its
parts on it.

A disposed part is faded on the **Parts** page, with **disposed** and the
date. Open it and choose **Recover gear** to bring it back.

A part that has never been on anything can be deleted for good: **Delete**
in its card's menu on the **Parts** page, or in the menu on its own page,
after a confirmation asking for the part's name.

## Example

You put a new front tire on your gravel bike. On the **Parts** page, click
**New** in the **Tire** box. The dialog is titled **New tire**: you call it
*Front One*, and it is a *Schwalbe One*, new tire day was today. **Create**.

The tire is on your shelf. Open it and choose **Attach**: the dialog is
titled **Attach tire Front One Schwalbe One**, and you pick **to** *front
wheel* **of** *Gravel Bike*, **at** today. **Attach**.

Open *Gravel Bike*. Its **Attached Parts** tab shows the tree: the
**Front wheel** card, and nested inside it the **Tire** card — *Front One*,
**since** today.

A year later the tread is gone. On the tree, open the **Tire** card's
**⋮** menu and choose **Replace tire**. The dialog is titled **New tire
for Front Wheel** and comes prefilled with the old tire's name and brand;
update the model and set the date. A switch reads **dispose old tire?** —
tick it — and click **Replace**.

The new tire is on the wheel. The old one is **disposed** on the **Parts**
page, and the **Attached to** panel on its page keeps the span: *Front
Wheel*, *front*, 12 May 2025 - 10 May 2026.
