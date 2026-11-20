# Getting started

TendaBike tracks the maintenance of your gear against how you actually ride, so
the first step is connecting the service where you log your rides. TendaBike
pulls in your gear and your riding history, and from then on keeps them up to
date by itself.

## Connect your account

Open TendaBike in your browser. If you're not signed in, you're taken to the
**About** page, where a **Connect your provider** section waits at the top —
one card per service the instance offers:

- **Strava** — "Rides you log in Strava"
- **intervals.icu** — "Training data from intervals.icu"

Pick the card for the service where you log your rides and click
**Connect Strava** (or **Connect intervals.icu**). You're taken to that
service, where you sign in and allow TendaBike to read your activities and
your gear. When you come back, you're in.

If the instance has no service set up, a banner reads
**Running offline — no activity providers configured.** and there are no cards
to click. Ask the person who runs your instance to connect a provider; that's
an admin task (see the [admin guide](../admin/index.md)).

## The welcome dialog

Right after you connect, a dialog opens: **Welcome to TendaBike!** It asks:

> Would you like to import your historic activities from Strava?

(intervals.icu, of course, by name.) Before you answer, TendaBike has already
pulled in your gear — the bikes it found on your account — and lists them
under **Your gear**. That's your first confirmation that the right account is
connected.

The dialog also tells you what the import does:

- It syncs **all** your past activities, so your maintenance tracking is
  based on your full riding history.
- It runs in the background and may take a few minutes depending on how many
  activities you have.
- In any case, the app will pull in new activities automatically.

Your choice:

- **Import Activities** starts the import. The button briefly reads
  **Importing...**, then the dialog closes and the import keeps running in the
  background.
- **Skip for Now** postpones the import — it's not cancelled. Pressing ESC
  does the same.

## After you choose

If you imported, there's nothing to do. The import runs in the background and
you can keep using the app; as activities arrive, they appear under
**Activities** in the menu at the top.

New rides are a separate matter from the historic import: the app pulls in
every new activity automatically — even if you skipped the import — and if
you change or delete an activity on the service side (in Strava or
intervals.icu), TendaBike follows along. You never have to sync by hand.

If you skipped, the import is waiting for you. Two ways to start it:

- the **Import Activities** button in the top bar — it's visible until your
  first activity has arrived; or
- your user menu (top right) → **Sync** → **Import Historic Activities** — the
  item is there for as long as the historic import hasn't run.

Either way the import starts running in the background.

If your gear page looks empty, TendaBike hasn't found any bikes on your
account (yet) — check that your gear is set up on the service side.

## Example

You've been riding on Strava for three years, and your account knows two
bikes. You click **Connect Strava**, sign in on Strava, allow the access, and
come back. The welcome dialog opens and shows **Your gear**: *Gravel Bike*
and *Road Bike* — the bikes from your Strava profile. You click
**Import Activities**.

The dialog closes. A few minutes later, **Activities** is filled with your
whole riding history, and the **Bikes** page — the first item in the menu —
shows your two bikes. Every ride you log on Strava from now on shows up here
on its own.
