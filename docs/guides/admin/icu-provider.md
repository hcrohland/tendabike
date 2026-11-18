# intervals.icu Provider — Admin Setup

How to register the Tendabike app with intervals.icu, configure webhooks, and
set the provider's environment variables. For the API research this is based
on, see [`../research/intervals-icu-api.md`](../research/intervals-icu-api.md).

## 1. Register the app

App registration is **manual** — there is no self-serve flow.

1. Log in to intervals.icu as the athlete who will own the app.
2. Apply at <https://intervals.icu/oauth/apply>.
3. Wait for approval. The app starts in **Pending** status; the OAuth flow
   (and with it everything downstream, including webhooks) does not work
   until intervals.icu approves it.

Once approved you get a **client ID** and **client secret**.

## 2. Configure the app (Manage App)

Open `/settings/apps` in intervals.icu, find the app, and click **Manage
App**. Set:

| Field | Value |
|---|---|
| Client ID / secret | From the approval (step 1). |
| Redirect URI | `https://<your-domain>/icu/token` (local dev: `http://localhost:8000/icu/token`) |
| Webhook URL | `https://<your-domain>/icu/callback` — **one URL per app**, shared by every athlete who authorizes it. Must be publicly reachable (HTTPS in production; a tunnel such as ngrok for local dev). |
| Webhook secret | Any long random string. This is the shared `secret` field in every webhook payload — copy it for `ICU_WEBHOOK_SECRET` (step 3). |
| Webhook event types | **`ACTIVITY_UPLOADED`**, **`ACTIVITY_UPDATED`**, and **`ACTIVITY_DELETED`** — the types Tendabike subscribes to. |
| PKCE | **On** — Tendabike's login flow always sends a PKCE code challenge (RFC 7636, S256); the option makes intervals.icu **enforce** it. Enable it for hardening. |

## 3. Environment variables

Set in the backend's environment (`.env` is loaded via `dotenvy`):

| Variable | Required | Notes |
|---|---|---|
| `ICU_CLIENT_ID` | yes | The app's client ID. The binary panics at startup without it. |
| `ICU_CLIENT_SECRET` | yes | The app's client secret. Panics at startup without it. |
| `ICU_WEBHOOK_SECRET` | **effectively yes** | Defaults to *empty*. The webhook verification is a constant-time compare against this value, so with it unset **every delivery is refused with 401**. Set it to the exact webhook secret from step 2. |

The redirect URI is derived from the instance's `HOSTNAME` variable
(see [the instance setup](index.md)): `https://<HOSTNAME>/icu/token`
(`http` for `localhost`/`127.0.0.1`; defaults to
`http://localhost:8000/icu/token`). The redirect URI in
step 2 needs to be set to this.

## 4. How it works

```
intervals.icu ──POST /icu/callback──▶ Tendabike
  { "secret": "…", "events": [ { "athlete_id", "type", "timestamp", "activity": { "id" } } ] }
```

1. The endpoint (`POST /icu/callback`) forwards the raw body to the provider
   layer, which verifies `secret` against `ICU_WEBHOOK_SECRET`
   (constant-time compare), drops the `TEST` delivery, and keeps only the
    subscribed activity events (`ACTIVITY_UPLOADED`, `ACTIVITY_UPDATED`,
    `ACTIVITY_DELETED`).
2. Each event's `athlete_id` is mapped to a TendaBike user. **The athlete must
   have already connected via the OAuth flow** (`/icu/login`); events for
   unknown athletes are refused.
3. Accepted events are queued in the `icu_events` table (each row carries the
   event's type) and the user's executor is woken. The executor drains the
   queue: `ACTIVITY_UPLOADED` / `ACTIVITY_UPDATED` fetch the full activity via
   `GET /activity/{id}` and import it; `ACTIVITY_DELETED` deletes the
   corresponding activity (and unregisters its part usage).
4. The endpoint answers `200` immediately; processing is asynchronous.

## 5. Troubleshooting

| Symptom | Cause / action |
|---|---|
| Every delivery gets **401** | `ICU_WEBHOOK_SECRET` is unset or does not match the Manage App webhook secret. Compare them byte-for-byte. |
| Deliveries re-fire repeatedly | Any non-2xx response makes intervals.icu re-fire the webhook with **exponential backoff**. Fix the underlying error (usually the 401 above); the retries stop once a delivery is accepted. |
| No webhook for a new activity | **Strava-sourced activities never produce webhooks** (Strava ToS). Check the activity's source in intervals.icu. |
| Activity imported but with no figures ("stub") | The activity is a stub (e.g. Strava-sourced with no data). Stubs are skipped and the user gets one batch notice with the skip count. |
| Events pile up in `icu_events` | Rate limiting: on HTTP 429 the current event **stays queued** and the queue pauses until the 15-minute window resets. OAuth app limits scale with user count (100/user/day, min 5,000, max 50,000; the 15-min limit is 1/8 of the daily). |
| A user's connection suddenly stops working | A 401 from the API (dead/revoked token — intervals.icu has no refresh tokens) **disables the user** and drops their queued events. The user must re-authorize via `/icu/login`. |
| Event refused: "Unknown icu athlete" | The athlete never completed the OAuth connect, so there is no mapping. Have them connect first. |

## 6. Verifying locally

With the backend running (`BIND_ADDR`, default `127.0.0.1:8000`), simulate a
delivery:

```bash
curl -i -X POST http://localhost:8000/icu/callback \
  -H 'Content-Type: application/json' \
  -d '{"secret":"<ICU_WEBHOOK_SECRET>",
       "events":[{"athlete_id":"<icu athlete id of a connected user>",
                  "type":"ACTIVITY_UPLOADED",
                  "timestamp":"2026-11-16T06:40:47.011+00:00",
                  "activity":{"id":"i55751783"}}]}'
```

- `200` (empty JSON body) — accepted and queued; the executor picks it up.
- `401` — secret mismatch (see step 3).
- `400` — malformed envelope/timestamp, or an unknown `athlete_id`.
- `500` — ingest failure; intervals.icu will retry.

Events for a connected user appear in the UI once the executor has processed
them (the activity is fetched from intervals.icu, so the activity must exist
and the API token must be valid).
