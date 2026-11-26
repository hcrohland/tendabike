# Local dev stack

Recipe for a full local stack (Postgres + backend + Vite dev server) for
real-browser verification with Playwright. Everything is throwaway: nothing
is written into the repo, and no real provider credentials are needed.

## Steps

1. **Reset the database.** From `backend/sqlx/`:

   ```bash
   DATABASE_URL=postgres://tendabike@localhost/tendabike_dev cargo sqlx database reset -y
   ```

   Drops and recreates the database and applies all migrations. Use a
   dedicated dev database: the `tendabike` database behind `DATABASE_URL`
   may carry a migration history that diverges from the checkout, and the
   reset is what wipes it. The `-y` is required in a non-interactive shell
   — without it, sqlx prompts for confirmation and fails. Done when: the
   last line reads `Applied …`.

2. **Build the backend.** From the repo root: `cargo build`. The binary is
   `target/debug/tendabike` — the Cargo workspace root is the repo root, not
   `backend/`.

3. **Run the backend** (in the background):

   ```bash
   mkdir -p /tmp/www
   DATABASE_URL=postgres://tendabike@localhost/tendabike_dev \
   STATIC_WWW=/tmp/www \
   STRAVA_CLIENT_ID=dummy STRAVA_CLIENT_SECRET=dummy \
   ICU_CLIENT_ID=dummy ICU_CLIENT_SECRET=dummy \
   ./target/debug/tendabike
   ```

   Provider credentials are optional: a provider is enabled iff its client id
   is set. Without any, the instance runs offline — `/api/providers` returns
   `[]` and the frontend shows the offline message instead of provider
   cards. The dummies above enable both providers, so the connect cards
   render for UI checks. `STATIC_WWW` may be any existing directory; Vite
   serves the frontend. Done when:
   `curl -s localhost:8000/api/providers` returns 200.

4. **Run the dev server** (in the background). From the repo root:
   `npm run dev`. Vite proxies `/api`, `/strava`, `/icu`, and `/source` to
   `:8000`. Its log floods with paraglide HMR reload lines — grep for
   `ready` and `error` rather than reading it. Done when:
   `curl -s localhost:5173/about` returns 200.

5. **Drive it with Playwright.** Playwright is installed globally
   (`/usr/lib/node_modules`), which ESM scripts outside the repo cannot
   resolve. Symlink it next to the script, then run the script with
   `import { chromium } from "playwright"`:

   ```bash
   ln -s /usr/lib/node_modules ./node_modules
   node script.mjs
   ```

   Check `page.on("console")` / `page.on("pageerror")` for the condition
   under test. Anonymous sessions log expected 401s (the app probes
   `/api/user` to detect them) — judge new errors, not their count.

## Teardown

Kill the backend by the pid listening on `:8000` (`ss -ltnp | grep 8000`)
— a `pkill -f` on the binary path also matches the shell that runs it.
The Vite server's process line does not contain "vite dev" — kill the pid
listening on `:5173` (`ss -ltnp | grep 5173`). Done when: both ports
refuse connections.
