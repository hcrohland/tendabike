use super::*;
use phf::{phf_map};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct PartTypeId(i32);

NewtypeDisplay! { () pub struct PartTypeId(); }
NewtypeFrom! { () pub struct PartTypeId(i32); }

/// List of of all valid part types.
///
/// We distingish main parts from spares:
/// - Main parts can be used for an activity - like a bike
/// - Spares can be attached to other parts and are subparts of main parts
#[derive(Clone, Debug, PartialEq)]
pub struct PartType {
    /// The primary key
    pub id: PartTypeId,
    /// The display name
    pub name: String,
    /// is it a main part? I.e. can it be used for an activity?
    pub main: PartTypeId,
    /// Part types that can be attached
    pub hooks: Vec<PartTypeId>,
    /// the order for displaying types
    pub order: i32,
    /// Potential group
    pub group: Option<String>
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ActTypeId(i32);

NewtypeDisplay! { () pub struct ActTypeId(); }
NewtypeFrom! { () pub struct ActTypeId(i32); }


static ACTIVITY_TYPES = phf::Map<&'static ActTypeId, &'static str> = phf_map! {
    "US" => "United States",
    "UK" => "United Kingdom",
};




/// The list of activity types
/// Includes the kind of gear which can be used for this activity
/// multiple gears are possible
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityType {
    /// The primary key
    pub id: ActTypeId,
    /// The name
    pub name: String,
    /// Gears which can be used for this activity type
    pub gear_type: PartTypeId,
}

impl ActTypeId {
    pub fn get(self, conn: &AppConn) -> TbResult<ActivityType> {
        Ok(activity_types::table
            .find(self)
            .first::<ActivityType>(conn)?)
    }
}

impl PartTypeId {

    /// get the full type for a type_id
    pub fn get (self, conn: &AppConn) -> TbResult<PartType> {
        unimplemented!()
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
    pub fn subtypes(self, conn: &AppConn) -> Vec<PartType> {
        use schema::part_types::dsl::*;
        let mut types = part_types
            .load::<PartType>(conn)
            .expect("Error loading parttypes");
        self.filter_types(&mut types)
    }

    /// Get the activity types valid for this part_type
    pub fn act_types(&self, conn: &AppConn) -> TbResult<Vec<ActTypeId>> {
        use schema::activity_types::dsl::*;

        Ok(activity_types
            .filter(gear.eq(self))
            .select(id)
            .get_results(conn)?)
    }
}

/// get all part types
#[get("/part")]
fn part(conn: AppDbConn) -> Json<Vec<PartType>> {
    Json(part_types::table
        .order(part_types::id)
        .load::<PartType>(&conn.0)
        .expect("error loading PartType"))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![part, activity]
}
