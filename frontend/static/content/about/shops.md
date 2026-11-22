# Using a shop

A **shop** is a place that services your gear. You can subscribe to a shop and
register your bikes and parts with it, so the people who service your gear can
see what you ride and its history. There are two sides to it: the rider who
subscribes, and the owner who runs the shop. Both are everyday users of
TendaBike — *running an instance* (managing accounts and providers) is a
separate, admin job covered in the
[admin guide](https://github.com/hcrohland/tendabike/blob/main/docs/guides/admin/index.md).

## Subscribing to a shop

Open your user menu (top right) and choose **Manage shops**. The **Shops**
page opens on the **My Subscriptions** tab. At the bottom, **Find Shops** has a
search box: type a name to look for shops on your instance. Each result is a
card — the shop's name, who runs it, and its description — with a
**Request Subscription** button.

Click **Request Subscription**: a dialog explains that, once approved, you can
register any of your bikes to the shop. Add a **Message (optional)** if you
like, and click **Send Request**. A shop owner can also send you a
**Registration Link**; opening it starts the same request for that shop.

## Your subscription requests

The **My Subscriptions** tab lists your requests in a table — **Shop**,
**Status**, **Date**, and **Actions**. The status is what the shop has decided:

- **Pending** — the shop hasn't answered yet. You can **Cancel** the request.
- **Active** — you're subscribed. You can register your gear (below) and
  **Unsubscribe**.
- **Rejected** — the shop turned you down. You can **Delete** the request.
- **Cancelled** — you withdrew it. There's nothing left to do.

## Registering your gear with a shop

Once a subscription is **Active**, register your gear with the shop so it's
visible to them. You can do it two ways:

- On the subscription's row, **Register bikes** (or the word for your sport)
  opens a list of your gear with a box beside each: tick one to register it,
  untick it to take it back.
- From any part's **⋮** menu: choose **Put into** and pick a shop. A part that
  is already registered with a shop shows **Move out of {the shop}** instead.

## A part belongs to the bike it's on

A part is owned by the bike it's on, so ownership moves as the shop works on
your gear:

- When the shop fits a part to your registered bike, it becomes **yours** — it's
  on your bike, so it counts against your plans. It also stays visible to the
  shop.
- When the shop moves that part off your bike onto another bike, it stops being
  yours — it belongs to that bike's owner now.

What's on your bike is yours; the shop sees what's on the bikes registered with
it.

## Unsubscribing

You can't unsubscribe while any of your gear is still registered with the shop.
The **Unsubscribe** button stays disabled, with a hint to unregister your parts
first. Untick your gear (as above), and the button opens: click **Unsubscribe**
and confirm.

## Shop mode

You can step into a shop to see things from its side. In your user menu, choose
**Enter {the shop}** — or **Enter shop** to pick from a list if you have
several. The app switches to that shop's view: a bordered frame with an
**Exit shop {the shop}** button at the top, and the **Parts** page shows only
the gear registered with that shop. Click **Exit shop {the shop}** to return to
your own view.

## Running a shop

Anyone can run a shop. In your user menu → **Manage shops**, the **My Shops**
tab is where you create and manage the shops you own.

- **Create Shop** (or **Create Your First Shop**, if you don't have one) opens
  a dialog with a **Shop Name**, a **Description (optional)**, and an
  **Automatically approve registration requests** switch. Click **Save**.
- Your shop's card shows its name, a count of the **parts** registered with it,
  and an **Owner** badge. Its **⋮** menu offers **Edit Shop**, **Delete Shop**,
  and the **Registration Link** — a copyable link you can send to riders.
- **Delete Shop** is only offered while no bikes are registered with the shop;
  the dialog warns that it can't be undone.

### Approving requests

Under your shop, **Pending Subscription Requests** lists the riders who've
asked to subscribe. For each one, **Respond** lets you add an optional message
and **Approve** or **Reject** it. With **Automatically approve registration
requests** on, requests are approved without you doing anything.

## Example

You run a small workshop and want your regulars to share their bikes with you.
In your user menu, **Manage shops** → **My Shops** → **Create Your First
Shop**: name it *Hub Stand*, add a description, and leave auto-approve off.
**Save**.

Copy the **Registration Link** from the shop's menu and send it to a rider.
They open it, add a note, and **Send Request**. Under *Hub Stand*,
**Pending Subscription Requests** shows their name — you **Respond** and
**Approve**.

On their side the subscription is now **Active**. They open **Register bikes**,
tick their *Gravel Bike*, and it's registered with *Hub Stand*. You **Enter Hub
Stand**; the **Parts** page shows their gravel bike and the parts on it. When
they're done, they untick the bike and **Unsubscribe**.
