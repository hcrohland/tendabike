# Intervals.icu API — Research

Findings for adding **intervals.icu as a second activity provider** to Tendabike (see map issue for the effort). Researched 2026-11-04/05 from the official forum threads and the RapiDoc/Swagger API docs. Source URLs cited per section.

## Sources

- API access thread: https://forum.intervals.icu/t/api-access-to-intervals-icu/609
- OAuth support thread: https://forum.intervals.icu/t/intervals-icu-oauth-support/2759
- API Integration Cookbook: https://forum.intervals.icu/t/intervals-icu-api-integration-cookbook/80090
- Server-side data model: https://forum.intervals.icu/t/server-side-data-model-for-scripts/25781
- Interactive docs (RapiDoc): https://intervals.icu/api-docs.html
- Swagger UI: https://intervals.icu/api/v1/docs/swagger-ui/index.html
- TypeScript data model (npm `@intervals-icu/js-data-model`): https://www.npmjs.com/package/@intervals-icu/js-data-model

## Base URL

`https://intervals.icu/api/v1`

## Authentication

Two methods exist; for a multi-user app use **OAuth 2.0**:

- **Authorization Code flow** (standard OAuth2). PKCE supported (`S256` only, optional per-app).
- **No refresh tokens.** Access tokens are long-lived and do not expire on a timer. A new authorization replaces the previous token. If the user revokes access, the token stops working (HTTP 401).
  - Source: OAuth thread, post 9 — "Intervals.icu is a bit easier because it doesn't use refresh tokens, only access tokens."
- **App registration is manual.** Apply at https://intervals.icu/oauth/apply (must be logged in as the owning athlete). The app starts in **Pending** status; the OAuth flow does not work until approved. Manage at `/settings/apps` → "Manage App" (client_id, secret, redirect URIs, webhooks).
- **Authorize endpoint:** `https://intervals.icu/oauth/authorize?client_id=...&redirect_uri=...&scope=...&state=...`
- **Token exchange** (within 2 minutes of the code): `POST https://intervals.icu/api/oauth/token` with `client_id`, `client_secret`, `code` [`&code_verifier`]. Response:
  ```json
  { "token_type": "Bearer", "access_token": "...", "scope": "ACTIVITY:READ", "athlete": { "id": "2049151", "name": "..." } }
  ```
- **Scopes** (comma-separated, `READ`/`WRITE` per scope): `ACTIVITY`, `WELLNESS`, `CALENDAR`, `CHATS`, `LIBRARY`, `SETTINGS`. We need `ACTIVITY:READ` (activities) — gear is under the athlete resource.
- **Revoke:** `DELETE /api/v1/disconnect-app` with `Authorization: Bearer <token>`.
- A **personal API key** (Basic auth, username `API_KEY`) also exists for single-user scripts — not suitable for a multi-user product, but useful for manual testing.

**Implication for us:** store the access token permanently (no refresh); on 401, prompt the user to re-authorize.

## Real-time updates (webhooks)

**Webhooks are available** (not polling-only).

- Configured on the app management page (`/settings/apps` → "Manage App"); select event types. **Per-app, one URL for all athletes** who authorized that app — we must map `athlete_id` in the payload to our users.
- Delivery: HTTP POST. Non-2xx responses are **re-fired with exponential backoff**.
- Payload:
  ```json
  { "secret": "ooKeodacie8I", "events": [ { "athlete_id": "i123456", "type": "ACTIVITY_UPLOADED", "timestamp": "...", "activity": { "id": "...", "start_date": "..." } } ] }
  ```
- The `secret` field verifies the webhook came from intervals.icu (constant-time compare).
- Event types: `ACTIVITY_UPLOADED` (file uploaded), `ACTIVITY_ANALYZED` (analysis complete, ~60s after upload — use this one), `CALENDAR_UPDATED`, `SPORT_SETTINGS_UPDATED`, `WELLNESS_UPDATED`.
- **Not delivered for Strava-sourced activities.**
- Recommended pattern: webhook as the change signal, then pull full data via `GET /activity/{id}`; periodic polling as a safety net.

## Endpoints

| Resource | Endpoint | Notes |
|---|---|---|
| List activities | `GET /athlete/{id}/activities?oldest=&newest=` | **Date-range required** (ISO `yyyy-MM-dd`). Summary data. `fields=` param selects fields. |
| Single activity | `GET /activity/{id}?intervals=true` | Full data; `intervals=true` adds interval breakdown. |
| Athlete | `GET /athlete/{id}` | Athlete profile/summary. |
| **List gear** | `GET /athlete/{id}/gear` | Gear items (see below). `.csv` suffix for CSV. |
| Wellness | `GET /athlete/{id}/wellness?oldest=&newest=` | Out of scope. |
| Activities CSV | `GET /athlete/{id}/activities.csv` | Bulk export. |

- **Athlete id `0`** = "the athlete associated with this token/API key."
- **Pagination is date-range based** (`oldest`/`newest`), not cursor/offset. Backfill = loop over date windows.

### Gear

`GET /api/v1/athlete/{id}/gear` returns an array of:

```json
{ "id": "gear-001", "type": "Bike", "name": "Road Bike", "distance": 5000000.0, "retired": false, "athlete_id": "12345" }
```

- `type` can be `"Bike"`, `"Shoes"`, etc. — **not just bikes**.
- `distance` is accumulated distance on the gear; `retired` marks decommissioned gear.
- Source: API access thread p.8 ("The gear data is available: GET /api/v1/athlete/{id}/gear"), `intervals-cli` crate `list_gear`.

## Rate limits

- Per **rolling 15-minute window** + **per day** (resets midnight UTC) + **10 req/s per IP**.
- Headers: `X-RateLimit-Limit: <15m>,<daily>`, `X-RateLimit-Remaining: <15m remaining>,<daily remaining>` — **same shape as Strava's** `X-RateLimit-*` headers (our `backend/strava/src/ratelimit.rs` may be reusable).
- Over limit: HTTP **429** + `Retry-After` (seconds).
- OAuth app limits scale with user count (100/user/day, min 5,000, max 50,000); 15-min limit = 1/8 of daily (min 2,500).

## Activity data model

Relevant fields on the activity object (list + single endpoints):

| Field | Type | Notes |
|---|---|---|
| `id` | String | e.g. `i55751783` — **a string, not an integer** (Strava ids are `i64`). |
| `name` | String | |
| `description` | String | |
| `type` | String | e.g. `"Ride"`, `"Run"`. |
| `start_date` / `start_date_local` | String | ISO-8601 UTC / local. |
| `distance` | Float | meters. (`icu_distance` — "use this one for distance".) |
| `moving_time` | Int | seconds. |
| `elapsed_time` | Int | seconds. |
| `total_elevation_gain` | Float | meters. **No `elevation_loss` field** (only gain; loss would need the altitude stream). |
| `device_name` | String | e.g. "Garmin Edge 530". |
| `gear` | Object | `{ "id": "..." }` — bike/shoe reference (id only; name via the gear endpoint). |
| `external_id` | String | id on the source platform (e.g. Garmin Connect). |
| `source` | Enum | `STRAVA`, `UPLOAD`, `MANUAL`, `GARMIN_CONNECT`, `OAUTH_CLIENT`, `DROPBOX`, `POLAR`, `SUUNTO`, `COROS`, `WAHOO`. |
| `file_type` | String | `fit` / `tcx` / `gpx`. |
| `calories` | Int | |
| `average_heartrate` / `max_heartrate` | Int | bpm. |
| `icu_average_watts` / `icu_weighted_avg_watts` | Int | avg / normalized power. |
| `icu_joules` | Int | total energy. |
| `icu_training_load` | Int | TSS. |
| `trainer` / `commute` / `race` | Bool | |
| `timezone` | String | Java timezone id. |

### `type` field — full enum

Source: `@intervals-icu/js-data-model` v1.2.0 (`ActivityType` union, "Generated from Intervals.icu source, do not edit"). The API declares `type?: string`, but this is the definitive set of values the system recognizes.

```
AlpineSki, BackcountrySki, Badminton, Canoeing, Crossfit, Cyclocross,
EBikeRide, EMountainBikeRide, Elliptical, Golf, GravelRide, Handcycle,
HighIntensityIntervalTraining, Hike, Hockey, IceSkate, InlineSkate,
Kayaking, Kitesurf, MountainBikeRide, NordicSki, OpenWaterSwim, Padel,
Pilates, Pickleball, Racquetball, Ride, RollerSki, Rowing, Rugby,
RockClimbing, Run, Sail, Skateboard, Snowboard, Snowshoe, Soccer,
Squash, StairStepper, StandUpPaddling, Surfing, Swim, TableTennis,
Tennis, TrackRide, TrailRun, Transition, Velomobile, VirtualRide,
VirtualRow, VirtualRun, VirtualSki, Walk, WaterSport, Wheelchair,
Windsurf, Workout, WeightTraining, Yoga, Other
```

## Key constraints / caveats

1. **No refresh tokens** — store the access token; handle 401 with re-authorization.
2. **Strava-sourced activities are blocked** via the API (Strava ToS). If a user's intervals.icu activities come from Strava, they appear as **stubs without figures**. A user who only imports from Strava will get little usable data from this API.
3. **Garmin data access at risk** (as of Oct 2026, Garmin's updated developer agreement) — monitor; Garmin-sourced activities may be blocked.
4. **Webhooks are per-app** (one URL) and **not delivered for Strava activities**.
5. **Date-range pagination** for backfill (no cursor).
6. **No public OpenAPI spec URL** found — the RapiDoc/Swagger UIs load it client-side; use the interactive UI or the npm TS data model as the schema reference.

## Mapping notes for Tendabike

- `Activity.id` is `i64` today and equals the Strava id; intervals.icu ids are strings → the id model must change (see the id-model ticket).
- `Part.source` (opaque provider gear id string) is the existing join for provider gear → reuse the same pattern for intervals.icu gear ids.
- `total_elevation_gain` maps to `Activity.climb`; there is no descend field (leave `descend` null or derive).
- `gear: {id}` on an activity joins to the gear endpoint's `id` → `Part.source`.
