use super::*;
use schema::{activity_types, part_types};
use domain::{PartType, PartTypeId};

/// List of of all valid part types.
///
/// We distingish main parts from spares:
/// - Main parts can be used for an activity - like a bike
/// - Spares can be attached to other parts and are subparts of main parts
#[derive(Clone, Debug, Serialize, Deserialize, Queryable, Identifiable, Associations, PartialEq)]
#[diesel(table_name = part_types)]
pub struct DieselPartType {
    /// The primary key
    pub id: i32,
    /// The display name
    pub name: String,
    /// is it a main part? I.e. can it be used for an activity?
    pub main: i32,
    /// Part types that can be attached
    pub hooks: Vec<i32>,
    /// the order for displaying types
    pub order: i32,
    /// Potential group
    pub group: Option<String>
}

impl From<DieselPartType> for PartType {
    fn from(d: DieselPartType) -> Self {

        let DieselPartType{id, name, main, hooks, order, group} = d;
        let hooks: Vec<PartTypeId> = hooks.into_iter().map(Into::into).collect();
        Self {id: id.into(), name, main: main.into(), hooks, order, group}
    }
}

/* 
/// The list of activity types
/// Includes the kind of gear which can be used for this activity
/// multiple gears are possible
#[derive(Debug, Clone, Identifiable, Queryable, PartialEq, Serialize, Deserialize)]
#[table("activity_types")]
pub struct D_ActivityType {
    /// The primary key
    pub id: ActTypeId,
    /// The name
    pub name: String,
    /// Gears which can be used for this activity type
    pub gear_type: PartTypeId,
}

impl ActTypeId {
    pub fn get(self, conn: &mut AppConn) -> AnyResult<ActivityType> {
        Ok(activity_types::table
            .find(self)
            .first::<ActivityType>(conn)?)
    }
}

impl PartTypeId {

    /// get the full type for a type_id
    pub fn get (self, conn: &mut AppConn) -> AnyResult<PartType> {
        use schema::part_types::dsl::*;
        Ok(part_types
            .find(self)
            .get_result::<PartType>(conn)?)
    }

    /// recursively look for subtypes to self in the PartType vector
    fn filter_types(self, types: &mut Vec<PartType>) -> Vec<PartType> {
        let mut res = types
            .drain_filter(|x| x.hooks.contains(&self) || x.id == self)
            .collect::<Vec<_>>();
        for t in res.clone().iter() {
            res.append(&mut t.id.filter_types(types));
        }
        res
    }

    /// get all the types you can attach - even indirectly - to this type_id
    pub fn subtypes(self, conn: &mut AppConn) -> Vec<PartType> {
        use schema::part_types::dsl::*;
        let mut types = part_types
            .load::<PartType>(conn)
            .expect("Error loading parttypes");
        self.filter_types(&mut types)
    }

    /// Get the activity types valid for this part_type
    pub fn act_types(&self, conn: &mut AppConn) -> AnyResult<Vec<ActTypeId>> {
        use schema::activity_types::dsl::*;

        Ok(activity_types
            .filter(gear.eq(self))
            .select(id)
            .get_results(conn)?)
    }
}

pub fn activities(conn: &mut AppConn) -> Vec<ActivityType> {
    activity_types::table
        .order(activity_types::id)
        .load::<ActivityType>(conn)
        .expect("error loading ActivityTypes")
}

pub fn parts(conn: &mut AppConn) -> Vec<PartType> {
    part_types::table
        .order(part_types::id)
        .load::<PartType>(conn)
        .expect("error loading PartType")
}
 */