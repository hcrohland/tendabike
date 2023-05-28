//! This module contains functions for redirecting to Strava URLs.
//!
//! The functions in this module are used to redirect users to Strava URLs for activities, gear, and users.
//!

use axum::{response::Redirect, extract::Path};

use crate::AppDbConn;

pub(super) async fn redirect_gear(Path(id): Path<i32>, mut conn: AppDbConn) -> Redirect {
    let uri = tb_strava::gear::strava_url(id, &mut conn).unwrap_or_else(|_| "/".to_string());
    Redirect::permanent(&uri)
}

pub(super) async fn redirect_act(Path(id): Path<i32>, mut conn: AppDbConn) -> Redirect {
    let uri = tb_strava::activity::strava_url(id, &mut conn).unwrap_or_else(|_| "/".to_string());
    Redirect::permanent(&uri)
}

pub(super) async fn redirect_user(Path(id): Path<i32>, mut conn: AppDbConn) -> Redirect {
    let uri = tb_strava::strava_url(id, &mut conn).unwrap_or_else(|_| "/".to_string());
    Redirect::permanent(&uri)
}