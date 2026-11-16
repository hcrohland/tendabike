# Strava Provider — Admin Setup

How to register the Tendabike app with Strava, set the provider's environment
variables, and create the webhook subscription. Official references:
[Webhook Events API](https://developers.strava.com/docs/webhooks) and
[Strava Developers](https://developers.strava.com/docs).

## 1. Register the app

Strava app registration is **self-serve** (no approval step, unlike
intervals.icu).

1. Log in to Strava and open <https://www.strava.com/settings/api>.
2. Create an application (name, website, and a description).
3. Copy the **Client ID** and **Client Secret** from the app's settings page.
4. Add the **Redirect URI**: `https://<your-domain>/strava/token` (local dev:
   `http://localhost:8000/strava/token`).

The app requests the scopes `read,activity:read_all,profile:read_all` and
**rejects logins where the authorized scope differs** — `activity:read_all`
also matters for webhooks: without it, activities set to *Only You* never
produce events.

## 2. Environment variables

Set in the backend's environment (`.env` is loaded via `dotenvy`):

| Variable | Required | Notes |
|---|---|---|
| `CLIENT_ID` | yes | The app's Client ID. The binary panics at startup without it. |
| `CLIENT_SECRET` | yes | The app's Client Secret. Panics at startup without it. |
| `REDIRECT_URL` | no | Defaults to `http://localhost:8000/strava/token`. Must match the Redirect URI in step 1. |

Token lifetime needs no admin work: the app stores each user's Strava refresh
token and refreshes the access token automatically (rotated refresh tokens are
persisted). See the troubleshooting table for the one edge case.

## 3. Create the webhook subscription

Strava allows **one subscription per app**; it then delivers events for every
athlete who authorized the app. Subscriptions are managed through the API, not
the settings page.

The callback is `https://<your-domain>/strava/callback` (publicly reachable;
HTTPS in production, a tunnel such as ngrok for local dev). Create the
subscription:

```bash
curl -X POST https://www.strava.com/api/v3/push_subscriptions \
  -F client_id=<CLIENT_ID> \
  -F client_secret=<CLIENT_SECRET> \
  -F callback_url=https://<your-domain>/strava/callback \
  -F verify_token=tendabike_strava
```

**`verify_token` must be exactly `tendabike_strava`** — it is hardcoded in the
backend (`backend/axum/src/strava/webhook.rs`), and the validation handshake
fails otherwise.

The request triggers the validation handshake: Strava issues a GET to the
callback URL with `hub.mode=subscribe`, a random `hub.challenge`, and the
`hub.verify_token`. The backend answers `200` with `{"hub.challenge":"…"}`
and the subscription is activated (the original POST then returns the
subscription's `id`). The callback must answer **within two seconds**.

Manage the subscription later with the same endpoint:

```bash
# View
curl -G https://www.strava.com/api/v3/push_subscriptions \
  -d client_id=<CLIENT_ID> -d client_secret=<CLIENT_SECRET>

# Delete
curl -X DELETE "https://www.strava.com/api/v3/push_subscriptions/<id>?client_id=<CLIENT_ID>&client_secret=<CLIENT_SECRET>"
```

## 4. How it works

```
Strava ──POST /strava/callback──▶ Tendabike
  { "object_type": "activity", "object_id": 1360128428, "aspect_type": "create",
    "updates": {}, "owner_id": 134815, "subscription_id": 120475, "event_time": 1516126040 }
```

1. Each delivery is a **single event** (no envelope, no per-delivery secret —
   the subscription's verify token is the authentication). The endpoint
   forwards the raw body to the provider layer.
2. The event's `owner_id` is mapped to a TendaBike user. **The athlete must
   have already connected via the OAuth flow** (`/strava/login`); events from
   unknown athletes are refused with `400`.
3. `activity` events are queued in the `strava_events` table and the user's
   executor is woken. The executor fetches the current activity from Strava
   and upserts it (`create`/`update`), or deletes it (`delete`). Only the
   latest queued event per activity is kept — Strava can send several events
   for one save (e.g. title and type change separately), and the dedup drops
   the older ones.
4. `athlete` events are app **deauthorizations** (`updates: {"authorized":"false"}`):
   the user is disabled (refresh token cleared) and nothing is queued.
5. The endpoint answers `200` immediately; processing is asynchronous.

In addition to webhooks, the queue carries **sync events** that walk
`GET /activities?after=<cursor>` from a point in time — these are what the
admin sync endpoints below enqueue.

## 5. Admin sync endpoints

For recovering missed events (downtime, rate-limit pauses) or backfilling:

| Endpoint | Auth | Effect |
|---|---|---|
| `GET /strava/sync?time=<unix>` | admin | Queues a sync from `time` for **all** non-disabled users and wakes their executors. `?user_id=<id>` narrows it to one user. |
| `GET /strava/sync/{id}?time=<unix>` | admin | Queues a sync from `time` for one user. |

`time` must not be in the future. The sync walks forward in pages of 25,
keeping a cursor at the last good activity, so an interrupted sync resumes
where it stopped.

## 6. Troubleshooting

| Symptom | Cause / action |
|---|---|
| Subscription creation fails | Most commonly: the callback did not answer the validation GET within 2 s, or did not echo `hub.challenge`. Test it: `curl 'https://<your-domain>/strava/callback?hub.verify_token=tendabike_strava&hub.challenge=test123&hub.mode=subscribe'` must return `200` with `{"hub.challenge":"test123"}`. Also: an app may only have **one** subscription — view and delete the old one first. A `verify_token` other than `tendabike_strava` fails the handshake. |
| No events for a specific activity | Check the authorizing scopes: *Only You* activities need `activity:read_all`. Check the subscription exists (View above). Check the endpoint returns `200` quickly — Strava retries a failed delivery **up to 3 attempts total**, then gives up. |
| Event refused: "Unknown event owner" | The athlete never completed the OAuth connect, so there is no mapping. Have them connect first. |
| Events pile up in `strava_events` | Rate limiting: on HTTP 429 the event **stays queued at its last good cursor** and the queue pauses until the window resets. Strava's read limits are 100 requests / 15 min and 1000 / day per user; the limiter is process-wide and in-memory (a restart resets it and re-learns from response headers). |
| A user's connection suddenly stops working | A 401 after a token refresh (revoked access, or a refresh token that expired — refresh tokens die 60 days after last use, which bites users whose Strava account sat unused) **disables the user** and drops their queued events. The user must re-authorize via `/strava/login`. |
| Missed a window of activities | Run the admin sync for the affected user(s) with `time` before the gap (section 5). |

## 7. Verifying locally

With the backend running (`BIND_ADDR`, default `127.0.0.1:8000`):

```bash
# 1. The validation handshake (what Strava does at subscription time):
curl 'http://localhost:8000/strava/callback?hub.verify_token=tendabike_strava&hub.challenge=test123&hub.mode=subscribe'
# → 200, body {"hub.challenge":"test123"}

# 2. A simulated activity delivery (owner_id = the Strava athlete id of a
#    connected user):
curl -i -X POST http://localhost:8000/strava/callback \
  -H 'Content-Type: application/json' \
  -d '{"aspect_type":"create","event_time":1763300000,
       "object_id":1360128428,"object_type":"activity",
       "owner_id":134815,"subscription_id":1,"updates":{}}'
```

- `200` (empty JSON body) — accepted and queued; the executor picks it up.
- `400` — malformed envelope, unknown `owner_id`, or a bad verify token on
  the GET handshake.
- `500` — ingest failure; Strava will retry (up to 3 attempts).

The activity appears in the UI once the executor has processed it (the
activity is fetched from Strava, so it must exist and the user's token must
be valid).
