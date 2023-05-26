use anyhow::ensure;
use async_session::log::{trace, info};
use axum::{Json, extract::{Query}};
use kernel::domain::{AnyResult, Error, Summary};

use tb_strava::event::{InEvent, process};
use serde_derive::{ Deserialize, Serialize};

use crate::{AppDbConn, ApiResult, user::{RUser, AxumAdmin}};

// complicated way to have query parameters with dots in the name
#[derive(Debug, Deserialize, Serialize)]
pub struct Hub {
    #[serde(rename = "hub.mode")]
    #[serde(skip_serializing)]
    mode: String,
    #[serde(rename = "hub.challenge")]
    challenge: String,
    #[serde(rename = "hub.verify_token")]
    #[serde(skip_serializing)]
    verify_token: String,
}

impl Hub {
    fn validate(self) -> AnyResult<Hub> {
        ensure!(
            self.verify_token == VERIFY_TOKEN, 
            Error::BadRequest(format!("Unknown verify token {}", self.verify_token))
        );
        ensure!(
            self.mode == "subscribe", 
            Error::BadRequest(format!("Unknown mode {}", self.mode))
        );
        Ok(self)
    }
}

const VERIFY_TOKEN: &str = "tendabike_strava";

// #[get("/hooks")]
pub(crate) async fn hooks (user: RUser, mut conn: AppDbConn) -> ApiResult<Summary> {
    let user = user.get_strava_user(&mut conn)?;
    user.lock(&mut conn)?;
    let res = process(&user, &mut conn).await?;
    user.unlock(&mut conn)?;
    Ok(Json(res))
}

// #[post("/callback", format = "json", data="<event>")]
pub(crate) async fn create_event(mut conn: AppDbConn, Json(event): axum::extract::Json<InEvent>) -> ApiResult<()> {
    trace!("Received {:#?}", event);
    event.convert()?.store(&mut conn)?;
    Ok(Json(()))
}

// #[get("/callback?<hub..>")]
pub(super) async fn validate_subscription (Query(hub): Query<Hub>) -> ApiResult<Hub> {
    info!("Received validation callback {:?}", hub);
    Ok(hub.validate().map(Json)?)
}

#[derive(Deserialize)]
pub(super) struct SyncQuery {
    time: i64,
    user_id: Option<i32>,
}

// #[get("/sync?<time>&<user_id>")]
pub(super) async fn sync_api (_u: AxumAdmin, mut conn: AppDbConn, Query(query): Query<SyncQuery>) -> ApiResult<()> {
    let user_id = query.user_id.map(|u| u.into());
    Ok(tb_strava::sync_users(user_id, query.time, &mut conn).await.map(Json)?)
}


// #[get("/sync/<tbid>")]
// pub(super) async fn sync(tbid: i32, _u: AxumAdmin, mut conn: AppDbConn, State(oauth): State<StravaClient>) -> ApiResult<Summary> {
//     let user = get_user_from_tb(tbid, oauth, &mut conn)?;
    
//     hooks(User(user), conn)
// }
