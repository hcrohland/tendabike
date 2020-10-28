use std::collections::HashMap;

use rocket::response::status;
use rocket_contrib::json::Json;

use self::schema::{part_types, parts};
use crate::user::*;
use crate::*;

use diesel::{self, QueryDsl, RunQueryDsl};

// make rls happy for now. This is broken anyways...

//#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub type Assembly = HashMap<PartId, Part>;

pub trait ATrait {
    fn part(&self, part: PartId) -> Option<&Part>;
}

impl ATrait for Assembly {
    fn part(&self, part: PartId) -> Option<&Part> {
        self.get(&part)
    }
}

/// The database's representation of a part.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Queryable,
    Identifiable,
    Associations,
    AsChangeset,
)]
#[primary_key(id)]
#[table_name = "parts"]
#[belongs_to(PartType, foreign_key = "what")]
pub struct Part {
    /// The primary key
    pub id: PartId,
    /// The owner
    pub owner: i32,
    /// The type of the part
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    /// purchase date
    pub purchase: DateTime<Utc>,
    /// usage time
    pub time: i32,
    /// Usage distance
    pub distance: i32,
    /// Overall climbing
    pub climb: i32,
    /// Overall descending
    pub descend: i32,
    /// usage count
    pub count: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Insertable)]
#[table_name = "parts"]
pub struct NewPart {
    /// The owner
    pub owner: i32,
    /// The type of the part
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    pub purchase: Option<DateTime<Utc>>,
}

#[derive(DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartId(i32);

NewtypeDisplay! { () pub struct PartId(); }
NewtypeFrom! { () pub struct PartId(i32); }

impl PartId {
    pub fn get(id: i32, user: &dyn Person, conn: &AppConn) -> TbResult<PartId> {
        PartId(id).checkuser(user, conn)
    }

    /// get the part with id part
    pub fn part(self, user: &dyn Person, conn: &AppConn) -> TbResult<Part> {
        let part = parts::table
            .find(self)
            .first::<Part>(conn)
            .with_context(|| format!("part {} does not exist", self))?;
        user.check_owner(
            part.owner,
            format!("user {} cannot access part {}", user.get_id(), part.id),
        )?;
        Ok(part)
    }

    /// get the name of the part
    ///
    /// does not check ownership. This is needed for rentals.
    pub fn name(self, conn: &AppConn) -> TbResult<String> {
        parts::table
            .find(self)
            .select(parts::name)
            .first::<String>(conn)
            .with_context(|| format!("part {} does not exist", self))
    }

    pub fn what(self, user: &dyn Person, conn: &AppConn) -> TbResult<PartTypeId> {
        Ok(self.part(user, conn)?.what)
    }

    /// check if the given user is the owner or an admin.
    /// Returns Forbidden if not.
    pub fn checkuser(self, user: &dyn Person, conn: &AppConn) -> TbResult<PartId> {
        use schema::parts::dsl::*;

        if user.is_admin() {
            return Ok(self);
        }

        let own = parts
            .find(self)
            .filter(owner.eq(user.get_id()))
            .select(owner)
            .first::<i32>(conn)?;
        if user.get_id() == own {
            return Ok(self);
        }

        bail!(Error::NotFound(format!(
            "user {} cannot access part {}",
            user.get_id(),
            self
        )))
    }

    /// apply a usage to the part with given id
    ///
    /// If the stored purchase date is later than the usage date, it will adjust the purchase date
    /// returns the changed part
    pub fn apply(self, usage: &Usage, conn: &AppConn) -> TbResult<Part> {
        use schema::parts::dsl::*;

        info!("Applying usage to part {}", self);
        // If the purchase date is newer than the usage, adjust it
        diesel::update(parts.find(self))
            .filter(purchase.gt(usage.start))
            .set(purchase.eq(usage.start))
            .execute(conn)?;
        Ok(diesel::update(parts.find(self))
            .set((
                time.eq(time + usage.time),
                climb.eq(climb + usage.climb),
                descend.eq(descend + usage.descend),
                distance.eq(distance + usage.distance),
                count.eq(count + usage.count),
            ))
            .get_result::<Part>(conn)?)
    }
}

fn allparts(user: &dyn Person, conn: &AppConn) -> TbResult<PartList> {
    use schema::parts::dsl::*;

    Ok(parts
        .filter(owner.eq(user.get_id()))
        .order_by(id)
        .load::<Part>(conn)?)
}

/// reset all usage counters for all parts of a person
///
/// returns the list of main gears affected
pub fn reset(user: &dyn Person, conn: &AppConn) -> TbResult<Vec<PartId>> {
    use schema::parts::dsl::*;
    use std::collections::HashSet;

    // reset all counters for all parts of this user
    let part_list = diesel::update(parts.filter(owner.eq(user.get_id())))
        .set((
            time.eq(0),
            climb.eq(0),
            descend.eq(0),
            distance.eq(0),
            count.eq(0),
        ))
        .get_results::<Part>(conn)?;

    // get the main types
    let mains: HashSet<PartTypeId> = part_types::table
        .select(part_types::id)
        .filter(part_types::main.eq(part_types::id))
        .load::<PartTypeId>(conn)
        .expect("error loading PartType")
        .into_iter()
        .collect();

    // only return the main parts
    Ok(part_list
        .into_iter()
        .filter(|x| mains.contains(&x.what))
        .map(|x| x.id)
        .collect())
}

impl NewPart {
    fn create(self, user: &User, conn: &AppConn) -> TbResult<Part> {
        use schema::parts::dsl::*;

        user.check_owner(
            self.owner,
            format!("user {} cannot create this part", user.get_id()),
        )?;

        let values = (
            owner.eq(self.owner),
            what.eq(self.what),
            name.eq(self.name),
            vendor.eq(self.vendor),
            model.eq(self.model),
            purchase.eq(self.purchase.unwrap_or_else(Utc::now)),
            time.eq(0),
            distance.eq(0),
            climb.eq(0),
            descend.eq(0),
            count.eq(0),
        );

        let part: Part = diesel::insert_into(parts).values(values).get_result(conn)?;
        Ok(part)
    }
}

#[get("/<part>")]
fn get(part: i32, user: &User, conn: AppDbConn) -> ApiResult<Part> {
    Ok(Json(PartId(part).part(user, &conn)?))
}

#[post("/", data = "<newpart>")]
fn post(
    newpart: Json<NewPart>,
    user: &User,
    conn: AppDbConn,
) -> Result<status::Created<Json<Part>>, ApiError> {
    let part = newpart.clone().create(user, &conn)?;
    let url = uri!(get: i32::from(part.id));
    Ok(status::Created(url.to_string(), Some(Json(part))))
}

#[get("/all")]
fn myparts(user: &User, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(attachment::for_parts(allparts(user, &conn)?,&conn))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get, post, myparts]
}
