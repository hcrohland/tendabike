# Planned maintenance

TendaBike tracks your parts against how you actually ride. You set a **service
plan** — how much a part should last before it needs service — and the app
counts your riding against it. As a part uses up its limit, a badge goes from
green to amber to red, and you know its service is due.

This page is about the badge and the numbers behind it, then how to set a plan,
what to do when a service comes due, and how logging a service — or replacing a
part — starts the clock over. The parts themselves are in [The
components](parts-assemblies.md).

## The due-ness badge

A part with a service plan carries a small badge that shows how close its
service is to being due; a part with no plan has none. You'll see it on the
part's card on the **Parts** page, in a bike's **Attached Parts** tree, and on
its service logs.

The badge is a percentage: how much of the plan's limit you've already used.
The higher it is, the closer the service is to being due. Hover it and it shows
the numbers — what you've used over the limit, like `4,800/5,000` for
kilometres.

The colour is the verdict:

- **Green** — the part is fine; it has a healthy share of its limit left.
- **Amber** — it's getting close: all but the last sliver of the limit is used.
- **Red** — the service is due. The part has used up, or gone past, its limit.

A bike's card on the **Bikes** page carries its own badge: a count of the
services on that bike that are amber or red, so you can see at a glance which
bike needs attention.

## What the badge counts

A plan is measured against the riding you log on the part, from the day you got
it — or from the last time you serviced it, once you have. The counters are the
same ones on the part's card:

- **rides** — how many activities.
- **hours** — riding time.
- **km** — distance.
- **climb** and **descend** — metres of climbing and descending.
- **energy** — the effort, in kilojoules, when your provider supplies it.

On top of those, a plan can be measured in **days** — a service that's due on a
schedule rather than on distance. A plan can use any of these, on its own or
combined, and the badge shows the closest of a part's limits to being due.

## Set a service plan

You set a plan from the **Service Plans** tab, and where you set it decides what
the plan follows.

**On a part**, it's for that specific part. Open the part, go to its
**Service Plans** tab, and click **add**: the dialog is titled **New service
plan for {the part's name}** and has a **Name** field and the limit fields
(below). The plan belongs to this one part and stays with it.

**On a bike**, it's for a *place* — a kind of part. Open the bike, go to its
**Service Plans** tab, and click **add**: the dialog is titled **New service
plan for …** and asks which part it is for — the kind, like **derailleur** or
**front tire** — **of** which bike, and a **Name** for the plan. For the bike,
pick this one or **any bike**, to cover that kind of part on every bike you
have.

Then set the limit(s): click **+** to add a unit — **days**, **rides**,
**hours**, **km**, **climb**, **descend**, **energy** — and type how much of it
the part should last. Add as many as you like, and **−** drops one. Click
**Save**.

The choice matters when a part is replaced: a plan for a *place* follows the
new part into the slot, while a plan for a *specific part* stays with the old
one.

The common case is a distance — *a derailleur every 5,000 km* — but a plan can
combine any of the counters, and a **days** limit suits a service that's due on
a schedule.

## When a service comes due

There is no separate alert; the badge is the alert. As a part uses up its limit
the badge goes green, then amber in the last sliver, then red when the service
is due. A red badge on a part — or a count on the bike's card — is TendaBike's
way of saying its service is due.

## Log a service

When the service is done, log it. Open the part (or the bike), go to the
**Service Logs** tab, and click **add**. The dialog is titled **New Service for
{the part}** — **New Service for Front One Schwalbe One**, say — and has:

- **Name** — what the service was, like *Chain cleaning*.
- **Notes** — anything to remember.
- **Resolves:** — the part's service plans as a tick list; tick the one(s) this
  service completes.
- **at** — the day it was done.

Click **Save**. A service that resolves a plan starts that plan's clock over:
the counters reset to zero from the service date, and the badge goes back to
green. The log entry keeps the window it just closed — the days and the riding
since the previous service — so a part's service history sits right there on the
tab.

Each logged service has a **⋮** menu: **Change Service**, **Repeat Service**
(the same service, dated again), and **Delete Service**.

## Replace a part

Replacing a part is in the part's card menu — **Replace {type}**, like
**Replace derailleur** — and it opens a **New {type} for {the bike}** dialog,
prefilled with the part being replaced, with a switch to **dispose** the old
one. For a part that carries others, a **Keep all attached parts** switch
decides whether the parts on it come along.

What happens to the plan depends on what it was set for:

- A plan set **for the place** on the bike follows the new part. The
  replacement sits in the same slot, so it's covered by the same plan from the
  day it goes on — and a fresh part has ridden nothing, so its badge starts
  green and climbs as you ride.
- A plan set **for the specific part** stays with the old part. The new part has
  no plan until you give it one.

## Example

Your gravel bike's rear derailleur is due an adjustment. On the bike's
**Service Plans** tab, click **add**: the kind is **derailleur**, **of**
*Gravel Bike*, the name is *Derailleur service*, and you add a **km** limit of
**5,000**. **Save**.

The derailleur's card now shows the badge at a low green percentage. Rides add
up, and one morning it's amber — `4,600/5,000`, you're close. A few days later
it's red: the service is due.

You take the bike in and get the derailleur adjusted. On its page,
**Service Logs** → **add**: name *Derailleur service*, tick **Resolves:
Derailleur service**, **at** today, **Save**. The badge snaps back to green —
the counter is now measuring from today.

A year later the derailleur is worn out. In its card menu, **Replace
derailleur**: the dialog is prefilled, you set the new model, tick **dispose
old derailleur?**, and click **Replace**. The new derailleur is on the bike and
already under the *Derailleur service* plan, its badge at zero and climbing —
because the plan was set for the place, not the old derailleur.
