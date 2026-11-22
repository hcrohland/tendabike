# Frontend routes

Navigation map: route → page-specific components → backend calls.
Shared widgets (badges, counter rows, generic dialogs) are not listed.

Every page is hydrated by the layout's one-time `GET /api/user` → `ApiRead::User`
(the user record plus the full `Summary`); pages then read that shared state, so
only the calls listed below are page-specific. `myfetch()` with no method is a
`GET`. Static markdown (`/content/*.md`) and `GET /api/types` are not backend-API
domain reads. The admin/provider calls (noted as such) are deliberately absent
from the `ApiRead`/`ApiWrite` catalog.

## /

- `ToyGroup.svelte` — lists the active category's gear as cards (active, then a disposed bin). Reads shared `parts` state; no direct calls.
- `GearCard.svelte` — one gear card (name, model, usage chips, provider source link). No direct calls.
- `PlanBadge.svelte` — per-gear service-plan alert indicator. No direct calls.

## /about

- `ContentPage.svelte` — renders the localized `about` markdown (`/content/about.*.md`, static).
- `ConnectProviders.svelte` — provider hero cards for anonymous visitors. Reads the shared provider state (the layout's one-time `GET /api/providers`); no direct calls.

## /activities

- `Activities.svelte` — derives the activity list from the route params (all, or the activities of one part / attachment). Reads shared `activities` state; no direct calls.
- `ActList.svelte` — date-range slider, gear filter, search, sort, and the activity cards. No direct calls.
- `TotalsCard.svelte` — totals chips (click to sort) plus the filter controls. No direct calls.
- `ActCard.svelte` — one activity card (metrics + edit menu).
- `ChangeActivity.svelte` — edit an activity's gear and metrics.
  - `PUT /api/activ/{id}` → `ApiWrite::ActivityUpdate`

## /activities/[...rest]

- `Activities.svelte` — same component as `/activities`; the `[...rest]` path pins a part (and optional start time). No direct calls.
- `ActList.svelte` — filter/sort controls and the activity cards. No direct calls.
- `TotalsCard.svelte` — totals chips and filter controls. No direct calls.
- `ActCard.svelte` — one activity card (metrics + edit menu).
- `ChangeActivity.svelte` — edit an activity's gear and metrics.
  - `PUT /api/activ/{id}` → `ApiWrite::ActivityUpdate`

## /admin

- `Admin.svelte` — the admin user list with bulk actions.
  - `GET /api/user/all` — admin stats (provider-driver op; not an `ApiRead` variant)
  - `GET /api/activ/rescan` — rescan all users (direct op; not an `ApiWrite` variant)
- `AdminCard.svelte` — per-user card (counts + action menu).
  - `GET /strava/sync/{id}` — queue a per-user sync (provider direct op)
  - `POST /strava/disable/{id}` — deauthorize a user (provider direct op)
- `CreateSync.svelte` — dialog to queue a sync from a date (all users, or one via `&user_id=`).
  - `GET /strava/sync?time=…` — trigger processing (batched admin op)
- `DeleteUser.svelte` — dialog to delete a user.
  - `DELETE /strava/delete/{id}` — delete a user (provider direct op)

## /cat

- `ToyGroup.svelte` — lists the active category's gear as cards (active, then a disposed bin). Reads shared `parts` state; no direct calls.
- `GearCard.svelte` — one gear card (name, model, usage chips, provider source link). No direct calls.
- `PlanBadge.svelte` — per-gear service-plan alert indicator. No direct calls.

## /data-protection

- `ContentPage.svelte` — renders the localized `data-protection` markdown (`/content/data-protection.*.md`, static).

## /part/[id]

- `Gear.svelte` — renders `Part.svelte` for the id, or a not-found alert.
- `Part.svelte` — the gear header (action menu) plus the attachment history and the part's tabs. No direct calls of its own (actions open the dialogs below).
- `Subparts.svelte` — the part's attached-parts tree.
- `PartCard.svelte` — one attached part (menu: log service, move/attach, replace).
  - `POST /api/service` → `ApiWrite::ServiceCreate` (log a service)
  - `POST /api/part/attach` → `ApiWrite::AttachmentAttach` (move/attach)
  - `POST /api/part` + `POST /api/part/attach` (→ `ApiWrite::PartCreate` + `ApiWrite::AttachmentAttach`; plus `POST /api/part/dispose` → `ApiWrite::AttachmentDispose` when the old part is disposed) — replace
- `Wizard.svelte` — batch-install a group of spare types onto the gear.
  - `POST /api/part` + `POST /api/part/attach` per part (→ `ApiWrite::PartCreate` + `ApiWrite::AttachmentAttach`)
- `PartHist.svelte` — the attachment history (per-attachment menu: remove).
  - `POST /api/part/detach` → `ApiWrite::AttachmentDetach`
- `PlanList.svelte` — the part's service plans.
- `PlanBlock.svelte` — one plan card (menu: change/delete plan) with per-gear rows.
  - `PUT /api/plan` → `ApiWrite::ServicePlanUpdate`
  - `DELETE /api/plan/{id}` → `ApiWrite::ServicePlanDelete`
- `PlanRow.svelte` — one plan's per-gear row (menu: new service, replace, install).
  - `POST /api/service` → `ApiWrite::ServiceCreate`
  - `POST /api/part` + `POST /api/part/attach` (→ `ApiWrite::PartCreate` + `ApiWrite::AttachmentAttach`; plus `POST /api/part/dispose` when disposing) — replace / install
- `ServiceList.svelte` — the part's service logs.
- `ServiceHist.svelte` — one service with its history chain.
- `ServiceMenu.svelte` — per-service actions.
  - `PUT /api/service` → `ApiWrite::ServiceUpdate`
  - `POST /api/service/redo` → `ApiWrite::ServiceRedo`
  - `DELETE /api/service/{id}` → `ApiWrite::ServiceDelete`
- `NoteList.svelte` — the part's notes.
- `NoteCard.svelte` — one note (menu: change/delete).
  - `DELETE /api/part/notes/{id}` → `ApiWrite::PartNoteDelete`
- `NewPart.svelte` — create a part.
  - `POST /api/part` → `ApiWrite::PartCreate`
- `ChangePart.svelte` — edit a part's details.
  - `PUT /api/part/{id}` → `ApiWrite::PartChange`
- `DeletePart.svelte` — delete a part.
  - `DELETE /api/part/{id}` → `ApiWrite::PartDelete`
- `DisposePart.svelte` — detach and/or dispose a part.
  - `POST /api/part/detach` → `ApiWrite::AttachmentDetach`
  - `POST /api/part/dispose` → `ApiWrite::AttachmentDispose`
- `RecoverPart.svelte` — recover a disposed part.
  - `POST /api/part/recover` → `ApiWrite::AttachmentRecover`
- `AttachPart.svelte` — attach a part to a gear.
  - `POST /api/part/attach` → `ApiWrite::AttachmentAttach`
- `InstallPart.svelte` — create and attach a new part.
  - `POST /api/part` → `ApiWrite::PartCreate`
  - `POST /api/part/attach` → `ApiWrite::AttachmentAttach`
- `PlanModal.svelte` — create/edit a service plan.
  - `POST /api/plan` → `ApiWrite::ServicePlanCreate`
  - `PUT /api/plan` → `ApiWrite::ServicePlanUpdate`
- `ServiceActions.svelte` — dispatcher that mounts the service create/edit/redo/delete dialogs.
- `NewNote.svelte` — create/edit a note (text or file).
  - `POST /api/part/{part}/notes` → `ApiWrite::PartNoteCreateText`
  - `POST /api/part/{part}/notes/file` → `ApiWrite::PartNoteCreateFile`
  - `PUT /api/part/notes/{id}` → `ApiWrite::PartNoteUpdateText`
  - `PUT /api/part/notes/{id}/file` → `ApiWrite::PartNoteUpdateFile`
  - `DELETE /api/part/notes/{id}/file` → `ApiWrite::PartNoteRemoveFile`
- `ShopRegistration.svelte` — register/unregister a part to a shop.
  - `POST /api/shop/{shop}/parts` → `ApiWrite::ShopRegisterPart`
  - `DELETE /api/shop/{shop}/parts/{part}` → `ApiWrite::ShopUnregisterPart`

## /plans

- `PlanList.svelte` — all service plans for the active category.
- `PlanBlock.svelte` — one plan card (menu: change/delete plan) with per-gear rows.
  - `PUT /api/plan` → `ApiWrite::ServicePlanUpdate`
  - `DELETE /api/plan/{id}` → `ApiWrite::ServicePlanDelete`
- `PlanRow.svelte` — one plan's per-gear row (menu: new service, replace, install).
  - `POST /api/service` → `ApiWrite::ServiceCreate`
  - `POST /api/part` + `POST /api/part/attach` (→ `ApiWrite::PartCreate` + `ApiWrite::AttachmentAttach`; plus `POST /api/part/dispose` when disposing) — replace / install

## /register/[shopid]

- `Register.svelte` — renders `Subscriptions.svelte` with a `shopid`, which auto-opens the subscription dialog.
- `Subscriptions.svelte` — the user's subscription table plus the shop search.
  - `GET /api/shop/subscriptions` → `ApiRead::ShopSubscriptions` (mine)
  - `GET /api/shop/{shopid}` → `ApiRead::Shop` (loads the shop to open the request dialog)
  - `DELETE /api/shop/subscriptions/{id}` → `ApiWrite::ShopSubscriptionCancel`
  - `POST /api/shop/{shop}/parts` → `ApiWrite::ShopRegisterPart` (register gear)
  - `DELETE /api/shop/{shop}/parts/{part}` → `ApiWrite::ShopUnregisterPart` (unregister gear)
- `ShopSearch.svelte` — search for other shops to subscribe to.
  - `GET /api/shop/search` → `ApiRead::ShopSearch`
- `SubscriptionRequestModal.svelte` — request a subscription to a shop.
  - `POST /api/shop/subscriptions` → `ApiWrite::ShopSubscriptionCreate`

## /shops

- `Shops.svelte` — tabs: my subscriptions / my shops.
- `Subscriptions.svelte` — the user's subscription table.
  - `GET /api/shop/subscriptions` → `ApiRead::ShopSubscriptions` (mine)
  - `DELETE /api/shop/subscriptions/{id}` → `ApiWrite::ShopSubscriptionCancel`
  - `POST /api/shop/{shop}/parts` → `ApiWrite::ShopRegisterPart`
  - `DELETE /api/shop/{shop}/parts/{part}` → `ApiWrite::ShopUnregisterPart`
- `ShopSearch.svelte` — search for other shops.
  - `GET /api/shop/search` → `ApiRead::ShopSearch`
- `ShopList.svelte` — the user's owned shops as cards.
- `ShopCard.svelte` — one shop card (owner, parts count).
  - `GET /api/shop/{id}/parts` → `ApiRead::ShopParts`
- `ShopOwnerMenu.svelte` — owner menu (edit, delete, copy registration link).
- `ShopModal.svelte` — create/edit a shop.
  - `POST /api/shop` → `ApiWrite::ShopCreate`
  - `PUT /api/shop/{id}` → `ApiWrite::ShopUpdate`
- `DeleteShop.svelte` — delete a shop.
  - `DELETE /api/shop/{id}` → `ApiWrite::ShopDelete`
- `ShopSubscriptions.svelte` — a shop's pending subscription requests (approve/reject).
  - `GET /api/shop/{id}/subscriptions` → `ApiRead::ShopSubscriptions` (the shop's)
  - `POST /api/shop/subscriptions/{id}/approve` → `ApiWrite::ShopSubscriptionApprove`
  - `POST /api/shop/subscriptions/{id}/reject` → `ApiWrite::ShopSubscriptionReject`

## /spares

- `Spares.svelte` — the active category's spare types.
- `SpareType.svelte` — one spare type (its parts, each with a menu).
  - `POST /api/part` → `ApiWrite::PartCreate` (new part)
  - `POST /api/part/attach` → `ApiWrite::AttachmentAttach` (attach)
  - `DELETE /api/part/{id}` → `ApiWrite::PartDelete`
  - `POST /api/part/detach` + `POST /api/part/dispose` (→ `ApiWrite::AttachmentDetach` + `ApiWrite::AttachmentDispose`) — dispose

## /stats

- `Statistics.svelte` — elevation / distance / time charts over the category's activities, with year, gear, and monthly toggles. Reads shared `activities` state; no direct calls.
