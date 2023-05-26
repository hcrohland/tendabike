use axum::{response::Redirect, extract::Path};

use crate::AppDbConn;

// #[get("/bikes/<id>")]
// #[debug_handler]
pub(super) async fn redirect_gear(Path(id): Path<i32>, mut conn: AppDbConn) -> Redirect {
    let uri = tb_strava::gear::strava_url(id, &mut conn).unwrap_or_else(|_| "/".to_string());
    Redirect::permanent(&uri)
}

// #[get("/activities/<id>")]
pub(super) async fn redirect_act(Path(id): Path<i32>, mut conn: AppDbConn) -> Redirect {
    let uri = tb_strava::activity::strava_url(id, &mut conn).unwrap_or_else(|_| "/".to_string());
    Redirect::permanent(&uri)
}
// #[get("/users/<id>")]
pub(super) async fn redirect_user(Path(id): Path<i32>, mut conn: AppDbConn) -> Redirect {
    let uri = tb_strava::strava_url(id, &mut conn).unwrap_or_else(|_| "/".to_string());
    Redirect::permanent(&uri)
}