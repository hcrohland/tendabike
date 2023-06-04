use s_diesel::schema;
use crate::AppConn;
use crate::event::Event;
use diesel_async::RunQueryDsl;
use anyhow::Result as AnyResult;


#[async_session::async_trait]
impl crate::Store for AppConn {
    async fn store_stravaevent(&mut self, e: Event) -> AnyResult<()> {
        diesel::insert_into(schema::strava_events::table)
            .values(&e)
            .get_result::<Event>(&mut self)
            .await?;
        Ok(())
    }
}
