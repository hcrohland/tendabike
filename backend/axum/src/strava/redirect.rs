//! The Strava admin redirect routes (ADR-0014): the deauthorization and
//! delete direct ops. The gear and user redirect routes moved to the
//! provider-neutral [`crate::redirect`] module, shared by the per-provider
//! route tables.

use axum::{
    Json,
    extract::{Path, State},
};
use tb_domain::UserId;

use crate::{ApiResult, AxumAdmin, appstate::AppState};

pub(super) async fn revoke_user(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    // The provider-agnostic direct op (ADR-0014): one transaction — the
    // provider layer's dispatch reads the target's real provider
    // in-transaction, builds the target's session in-txn, then deauthorizes
    // (no wake, no message; the existing queue race is preserved). A
    // per-provider route acting on a user of the other provider follows the
    // stated identity rule. The admin's acting session rides along (carried
    // for the admin-verification `tb_provider::strava` change).
    state
        .provider
        .deauthorize(Box::new(admin.session()), tbid)
        .await?;
    Ok(Json(()))
}

pub(super) async fn deleteuser(
    admin: AxumAdmin,
    Path(tbid): Path<UserId>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    // The provider-agnostic direct op (ADR-0014): one transaction — the
    // provider layer's dispatch reads the target's real provider
    // in-transaction, builds the target's session in-txn, then deletes (no
    // wake, no message; the existing queue race is preserved). A per-provider
    // route acting on a user of the other provider follows the stated
    // identity rule. The admin's acting session rides along (carried for the
    // admin-verification `tb_provider::strava` change).
    state
        .provider
        .delete_user(Box::new(admin.session()), tbid)
        .await?;
    Ok(Json(()))
}
