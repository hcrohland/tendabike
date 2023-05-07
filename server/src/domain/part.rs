use std::collections::HashMap;

use self::schema::{part_types, parts};
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
    /// last time it was used
    pub last_used: DateTime<Utc>,
    /// Was it disposed? If yes, when?
    pub disposed_at: Option<DateTime<Utc>>,
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

#[derive(Clone, Debug, PartialEq, Deserialize, AsChangeset)]
#[table_name = "parts"]
#[changeset_options(treat_none_as_null = "true")]
pub struct ChangePart {
    pub id: PartId,
    /// The owner
    pub owner: i32,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    pub purchase: DateTime<Utc>,
    /// Was it disposed? If yes, when?
    pub disposed_at: Option<DateTime<Utc>>,
}

#[derive(DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartId(i32);

NewtypeDisplay! { () pub struct PartId(); }
NewtypeFrom! { () pub struct PartId(i32); }

impl PartId {
    pub fn new (id:i32) -> PartId {
        PartId(id)
    }
    
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
            .first(conn)
            .with_context(|| format!("part {} does not exist", self))
    }

    pub fn what(self, conn: &AppConn) -> TbResult<PartTypeId> {
        parts::table
            .find(self)
            .select(parts::what)
            .first(conn)
            .with_context(|| format!("part {} does not exist", self))
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
    pub fn apply_usage(self, usage: &Usage, start: DateTime<Utc>, conn: &AppConn) -> TbResult<Part> {
        use schema::parts::dsl::*;

        trace!("Applying usage {:?} to part {}", usage, self);

        Ok(conn.transaction(|| {
            let part: Part = parts.find(self).for_update().get_result(conn)?;
            diesel::update(parts.find(self))
                .set((
                    time.eq(time + usage.time),
                    climb.eq(climb + usage.climb),
                    descend.eq(descend + usage.descend),
                    distance.eq(distance + usage.distance),
                    count.eq(count + usage.count),
                    purchase.eq(min(part.purchase, start)),
                    last_used.eq(max(part.last_used, start))
                ))
                .get_result::<Part>(conn)
        })?)
    }
}

impl Part {
    pub fn get_all(user: &dyn Person, conn: &AppConn) -> TbResult<Vec<Part>> {
        use schema::parts::dsl::*;

        Ok(parts
            .filter(owner.eq(user.get_id()))
            .order_by(last_used)
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
                last_used.eq(purchase)
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
}

impl NewPart {
    pub fn create(self, user: &dyn Person, conn: &AppConn) -> TbResult<Part> {
        use schema::parts::dsl::*;
        info!("Create {:?}", self);

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
            last_used.eq(self.purchase.unwrap_or_else(Utc::now)),
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

impl ChangePart {
    pub fn change(&self, user: &User, conn: &AppConn) -> TbResult<Part> {
        use schema::parts::dsl::*;
        info!("Change {:?}", self);

        user.check_owner(
            self.owner,
            format!("user {} cannot create this part", user.get_id()),
        )?;

        let part: Part = diesel::update(parts.filter(id.eq(self.id))).set(self).get_result(conn)?;
        Ok(part)
    }
}