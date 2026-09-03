# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Project Overview

**TendaBike** - A bike maintenance tracker that syncs with Strava. Users track parts (bike components), schedule services, and log cycling activities imported from Strava.

**Tech Stack**:

- **Frontend**: Svelte 5 + TypeScript + Vite + Tailwind CSS v4 (see [`frontend/AGENTS.md`](frontend/AGENTS.md))
- **Backend**: Rust 2024 + Axum 0.8 + sqlx 0.8 + PostgreSQL (see [`backend/AGENTS.md`](backend/AGENTS.md))

## Commands

### Full Project

- `npm run dev` - Start frontend dev server (Vite proxy → backend on `:8000`)
- `npm run build` - Production frontend build → `frontend/dist/`
- `npm run check` - Type checking (paraglide compile + svelte-check)
- `npm run format` - Format with Prettier
- `npm run fmtcheck` - Format check (used in CI)
- `cargo run` - Start backend server (listens on `BIND_ADDR`, serves frontend static files)
- `SQLX_OFFLINE=true cargo build` - Build backend (requires precompiled sqlx queries)

### Docker Build

- `docker build -t tendabike .` - Full image (Rust + Node multi-stage)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend (Svelte 5 + TypeScript)                           │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────────┐  │
│  │ Components  │──▶│ lib/store.ts │──▶│ myfetch() API   │  │
│  │ (.svelte)   │   │ lib/*.ts     │   │ proxy: /^api|   │  │
│  │             │   │ mapable()    │   │ strava/ → :8000 │  │
│  └─────────────┘   └──────────────┘   └─────────────────┘  │
└────────────────────────┬────────────────────────────────────┘
                         │ HTTP REST API
                         ▼
┌─────────────────────────────────────────────────────────────┐
│  Backend (Rust + Axum)                                      │
│                                                             │
│  ┌──────────┐    ┌──────────────────────────────┐          │
│  │ app/     │───▶│ axum/ (web layer)            │          │
│  │ binary   │    │ /api/{user,part,service...}  │          │
│  │          │    │ /strava/* (OAuth, webhook)   │          │
│  └──────────┘    └──────────┬───────────────────┘          │
│                             │                              │
│              ┌──────────────▼──────────────┐               │
│              │ domain/ (business logic)    │               │
│              │ Entities + Traits (Store)   │               │
│              └──────────────┬──────────────┘               │
│                             │                              │
│              ┌──────────────▼──────────────┐               │
│              │ sqlx/ (PostgreSQL)          │               │
│              │ tb_strava/ (Strava API)     │               │
│              └─────────────────────────────┘               │
└─────────────────────────────────────────────────────────────┘
                         │
                         ▼
              ┌──────────────────┐
              │ PostgreSQL DB    │
              │ tendabike schema │
              └──────────────────┘
```

## Key Entities (Shared Domain)

| Entity   | Backend                                                                      | Frontend                                          |
| -------- | ---------------------------------------------------------------------------- | ------------------------------------------------- |
| User     | [`domain/src/entities/user.rs`](backend/domain/src/entities/user.rs)         | [`lib/user.ts`](frontend/src/lib/user.ts)         |
| Part     | [`domain/src/entities/part.rs`](backend/domain/src/entities/part.rs)         | [`lib/part.ts`](frontend/src/lib/part.ts)         |
| Activity | [`domain/src/entities/activity.rs`](backend/domain/src/entities/activity.rs) | [`lib/activity.ts`](frontend/src/lib/activity.ts) |
| Service  | [`domain/src/entities/service.rs`](backend/domain/src/entities/service.rs)   | [`lib/service.ts`](frontend/src/lib/service.ts)   |
| Shop     | [`domain/src/entities/shop.rs`](backend/domain/src/entities/shop.rs)         | [`lib/shop.ts`](frontend/src/lib/shop.ts)         |
| Usage    | [`domain/src/entities/usage.rs`](backend/domain/src/entities/usage.rs)       | [`lib/usage.ts`](frontend/src/lib/usage.ts)       |

## API Endpoints

Base path: `/api` (proxied from frontend via Vite dev server)

| Resource | Routes                                      | Notes                      |
| -------- | ------------------------------------------- | -------------------------- |
| User     | `GET /user/`, `/summary`, `/all`, `/export` | Session auth via Strava    |
| Types    | `GET /types/*`                              | Part types, activity types |
| Shop     | `/shop/*`                                   | Shop management            |
| Part     | `/part/*`                                   | Bike parts, gear           |
| Part     | `/part/*` (attachments)                     | Part attachments (nested)  |
| Service  | `/service/*`                                | Maintenance services       |
| Plan     | `/plan/*`                                   | Service plans              |
| Activity | `/activ/*`                                  | Strava activities          |

Strava OAuth: `/strava/*` (OAuth flow, webhook endpoints)

## Data Flow

1. **Strava Sync**: Backend pulls activities from Strava API → stores in PostgreSQL
2. **Activity Import**: Activities linked to parts (gear); usage tracked via `usages` table
3. **Service Plans**: Scheduled maintenance based on part usage (time/distance/effort)
4. **Shop Delegation**: Parts can be assigned to shops for delegated maintenance
5. **Frontend State**: Svelte stores with `mapable()` pattern; auto-syncs via `myfetch()` polling

## Environment Variables

| Variable       | Default                          | Description                    |
| -------------- | -------------------------------- | ------------------------------ |
| `DB_URL`       | `postgres://localhost/tendabike` | PostgreSQL connection string   |
| `BIND_ADDR`    | `127.0.0.1:8000`                 | Backend listen address         |
| `STATIC_WWW`   | `../../frontend/dist`            | Frontend static files path     |
| `SQLX_OFFLINE` | (required)                       | Must be `true` for cargo build |

## Commit Rules

- **Never commit without reviews** by a subagent and the user
- **Always run `cargo fmt` and `npm run format` before committing** — the pre-commit hook enforces formatting, type checking, and linting. Never bypass it with `--no-verify` or `-n`.
- **Never skip commit hooks** — the pre-commit hook runs `cargo fmt --check`, `cargo sqlx prepare --check`, `cargo clippy`, `cargo check`, and frontend checks (`npm run fmtcheck`, `npm run check:ci`, `npm run build`). Bypassing them risks committing unformatted, broken, or unbuildable code.
- If formatting fails, run `cargo fmt` (backend) and `npm run format` (frontend) from the project root, then re-stage.

## Important Notes

- **No test framework** on either side - CI only runs type checking and format validation
- **Migrations**: Auto-run on backend startup via sqlx migrate; see [`backend/sqlx/migrations/`](backend/sqlx/migrations/)
- **i18n**: Frontend uses `@inlang/paraglide-js`; messages in `frontend/messages/{de,de_CH,en}.json`
- **License**: AGPL v3 - see [`LICENSE`](LICENSE)
- **Session management**: `tower-sessions` with PostgreSQL store; 10-day inactivity expiry
- **Frontend-backend coupling**: Frontend expects API at `/api/*` and `/strava/*`; backend serves both API and static files in production
