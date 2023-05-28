/* 
    tendabike - the bike maintenance tracker
    
    Copyright (C) 2023  Christoph Rohland 

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.

 */

 use anyhow::ensure;
use async_session::log::{trace, info};
use axum::{Json, extract::{Query, Path, State}};
use kernel::{domain::{AnyResult, Error, Summary}};

use tb_strava::{event::{InEvent, process}, StravaUser};
use serde_derive::{ Deserialize, Serialize};

use crate::{AppDbConn, ApiResult, user::{RUser, AxumAdmin}};

use super::refresh_token;

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

pub(crate) async fn hooks (user: RUser, mut conn: AppDbConn) -> ApiResult<Summary> {
    let user = user.get_strava_user(&mut conn)?;
    user.lock(&mut conn)?;
    let res = process(&user, &mut conn).await?;
    user.unlock(&mut conn)?;
    Ok(Json(res))
}

pub(crate) async fn create_event(mut conn: AppDbConn, Json(event): axum::extract::Json<InEvent>) -> ApiResult<()> {
    trace!("Received {:#?}", event);
    event.convert()?.store(&mut conn)?;
    Ok(Json(()))
}

pub(super) async fn validate_subscription (Query(hub): Query<Hub>) -> ApiResult<Hub> {
    info!("Received validation callback {:?}", hub);
    Ok(hub.validate().map(Json)?)
}

#[derive(Deserialize)]
pub(super) struct SyncQuery {
    time: i64,
    user_id: Option<i32>,
}

pub(super) async fn sync_api (_u: AxumAdmin, mut conn: AppDbConn, Query(query): Query<SyncQuery>) -> ApiResult<()> {
    let user_id = query.user_id.map(|u| u.into());
    Ok(tb_strava::sync_users(user_id, query.time, &mut conn).await.map(Json)?)
}


pub(super) async fn sync(Path(tbid): Path<i32>, _u: AxumAdmin, mut conn: AppDbConn, State(oauth): State<super::StravaClient>) -> ApiResult<Summary> {
    let conn = &mut conn;
    let user = StravaUser::read(tbid.into(), conn)?;
    let user = refresh_token(user, oauth, conn).await?;
    user.lock( conn)?;
    let res = process(&user, conn).await?;
    user.unlock(conn)?;
    Ok(Json(res))
}
