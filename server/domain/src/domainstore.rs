use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use s_diesel::schema;
use crate::ActTypeId;
use crate::ActivityType;
use crate::AnyResult;
use crate::AppConn;

use crate::PartType;
use crate::PartTypeId;

#[async_session::async_trait]
impl crate::Store for AppConn {
    async fn get_all_parttypes_ordered(&mut self) -> Vec<PartType> {
        use schema::part_types;
        part_types::table
            .order(part_types::id)
            .load::<PartType>(self)
            .await
            .expect("error loading PartType")
    }
    
    async fn get_parttype_by_id(&mut self, pid: PartTypeId) -> AnyResult<PartType> {
        // parttype_get
        use schema::part_types::dsl::*;
        Ok(part_types.find(pid).get_result::<PartType>(self).await?)
    }

    async fn get_activity_types_by_parttypeid(&mut self, tid: &PartTypeId) -> AnyResult<Vec<ActTypeId>> {
        use schema::activity_types::dsl::*;

        Ok(activity_types
            .filter(gear.eq(tid))
            .select(id)
            .get_results(self)
            .await?)
    }


    async fn get_all_activitytypes_order(&mut self) -> Vec<ActivityType> {
        use schema::activity_types;
        activity_types::table
            .order(activity_types::id)
            .load::<ActivityType>(self)
            .await
            .expect("error loading ActivityTypes")
    }

}