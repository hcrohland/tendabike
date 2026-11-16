# Tendabike — Admin Guide

Setup and operations for running a Tendabike instance: the instance-level
configuration first, then the per-provider setup.

## Instance setup

Tendabike is a single backend binary that serves the API and the built
frontend.

| Variable | Required | Notes |
|---|---|---|
| `HOSTNAME` | no | The instance's public host, with port where relevant (e.g. `tendabike.example.com`, `localhost:8000`). Both providers' redirect URLs are derived from it: `https://<HOSTNAME>/strava/token` and `https://<HOSTNAME>/icu/token` — `http` when the host is `localhost` or `127.0.0.1`. Defaults to the local-dev URLs (`http://localhost:8000/…`). The redirect URIs registered with the providers must match. |
| `DATABASE_URL` | no | PostgreSQL connection string. Defaults to `postgres://localhost/tendabike`. Apply migrations with `cargo sqlx migrate run` (from `backend/sqlx/`). `DB_URL` is a deprecated alias: it still works when `DATABASE_URL` is unset, but the binary prints a deprecation warning at startup. |
| `BIND_ADDR` | no | Listen address. Defaults to `127.0.0.1:8000` — set a public address (behind your TLS terminator) for a real deployment. |
| `STATIC_WWW` | no | Directory the frontend is served from. Defaults to `frontend/build` — build the frontend first (`npm run build` from `frontend/`). |
| `CSRF_SECRET` | no | |

The provider credentials below are additional: **both providers are required
at startup** — the binary panics if either set is missing, even if you only
intend to use one.

## Provider setup

Each provider needs its own app registration, environment variables, and
webhook configuration. The guides cover registration, configuration, how the
webhook flow works, and troubleshooting:

- [Strava provider](strava-provider.md)
- [intervals.icu provider](icu-provider.md)
