use crate::{ActTypeId, ActivityType, PartType, PartTypeId, TbResult};

/// A trait defining methods for accessing and manipulating types in the system.
#[async_trait::async_trait]
pub trait TypesStore {
    /// Returns a vector of all `PartType`s in the system, ordered by some criteria.
    async fn get_all_parttypes_ordered(&mut self) -> Vec<PartType>;

    /// Returns the `PartType` with the given `PartTypeId`.
    ///
    /// # Arguments
    ///
    /// * `pid` - The `PartTypeId` of the `PartType` to retrieve.
    ///
    /// # Returns
    ///
    /// The `PartType` with the given `PartTypeId`, or an `TbResult::Err` if it does not exist.
    async fn get_parttype_by_id(&mut self, pid: PartTypeId) -> TbResult<PartType>;

    /// Returns a vector of `PartTypeId`s for all `PartType`s that are maingear.
    async fn parttypes_all_maingear(&mut self) -> TbResult<Vec<PartTypeId>>;

    /// Returns a vector of `ActTypeId`s for all `ActivityType`s associated with the given `PartTypeId`.
    ///
    /// # Arguments
    ///
    /// * `tid` - The `PartTypeId` to retrieve `ActivityType`s for.
    ///
    /// # Returns
    ///
    /// A vector of `ActTypeId`s for all `ActivityType`s associated with the given `PartTypeId`, or an `TbResult::Err` if the `PartTypeId` does not exist.
    async fn get_activity_types_by_parttypeid(
        &mut self,
        tid: &PartTypeId,
    ) -> TbResult<Vec<ActTypeId>>;

    /// Returns a vector of all `ActivityType`s in the system, ordered by id.
    async fn activitytypes_get_all_ordered(&mut self) -> Vec<ActivityType>;
}
