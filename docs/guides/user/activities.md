# Your rides

Your rides — the activities you log on Strava or intervals.icu — are the
numbers TendaBike works from. Every [service plan](service-plans.md) counts
your riding against a limit, and that riding comes from your activities: the
kilometres, the hours, the climbing. TendaBike pulls your activities in on its
own, so this page is mostly about what to look at, and the few things you can
do by hand.

## They sync on their own

The sync is automatic and in the background. When you log a ride on your
provider, the provider lets TendaBike know, and the activity lands here without
anything for you to do. New rides appear on their own, and if you edit or
delete an activity on the provider side, TendaBike follows along. In everyday
use you never sync by hand.

The **Activities** page updates as rides come in: a ride you finish shows up a
little while later, on its own. (Your *first* import — pulling your whole
history — is in [Getting started](onboarding.md).)

## The Activities page

The **Activities** item in the top menu opens the page, titled **All
activities**. A summary of everything on the page sits at the top, with the
controls to narrow it down; below it is the list, one card per ride, most
recent first.

**The summary** adds up the rides currently shown and lets you filter them:

- A row of counters — **rides** (how many), time, **km**, climbing,
  descending, and, when your provider supplies it, **energy** — the total for
  the rides in view.
- A filter for one bike's rides, and a **Search** box to find a ride by name
  or device.
- A date slider: drag its two handles to look at a window of dates, and the
  counters and the list follow.
- Click any counter to sort the list by it — a small arrow shows the
  direction, and clicking again flips it.

**The list** is one card per ride:

- Its date and time, then its name — a link back to the ride on your provider
  (hover the small icon for **View on Strava** or **View on intervals.icu**).
- The device it was logged with, on the right.
- The ride's numbers: which **bike** it was on (a link to the bike), its time,
  **km**, climbing, descending, and energy when there is some.
- A **⋮** menu with **Change**.

Before your first activity has arrived, the page shows **No activities**.

## Change a ride

If a ride landed on the wrong bike, or a number is missing or off, open its
**⋮** menu and choose **Change**. The dialog is titled **Change Activity** and
shows the ride's name and its time, a picker for which **bike** it was on, and
the numbers — **Climb (m)**, **Descend (m)**, **Distance (m)**, **Time (sec)**,
and **Duration (sec)**. Fix what you need and click **Update**.

## Re-sync and pull historic rides

You don't need to, in the everyday — the sync runs itself. The **Sync** menu
in your user menu (top right) is there for the moments you do want to nudge
it:

- **Refresh data** — pull a fresh look at your activities now. Reach for it if
  the list looks short; it's a fix, not a habit.
- **Import Historic Activities** — your whole riding history, offered for as
  long as you haven't imported it. See [Getting started](onboarding.md) for
  the first import and the **Import Activities** button in the top bar.
- **With CSV File** — for riders who track with a Garmin device. Upload a CSV
  exported from Garmin Connect and TendaBike adds the data your device
  recorded — such as descent — to the rides it can match by start time; rides
  it can't match are skipped and listed.

## Example

You ride your gravel bike 40 km this morning and log it on Strava. A little
while later it's on the **Activities** page — on *Gravel Bike*, its
kilometres added to the **km** total at the top.

A week later you spot a ride from last month on the wrong bike: you'd ridden
the road bike, not the gravel one. Open the ride's **⋮** menu, choose
**Change**, set the bike to *Road Bike*, and click **Update**. The ride is now
on the road bike, and each bike's counters follow.
