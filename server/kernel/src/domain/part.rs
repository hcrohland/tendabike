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

 use std::cmp::{min, max};

use super::*;
use ::time::OffsetDateTime;
use schema::{part_types, parts};

// make rls happy for now. This is broken anyways...

//#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub type Assembly = HashMap<PartId, Part>;

trait ATrait {
    fn part(&self, part: PartId) -> Option<&Part>;
}

impl ATrait for Assembly {
    fn part(&self, part: PartId) -> Option<&Part> {
        self.get(&part)
    }
}

/// The database's representation of a part.
#[serde_as]
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
#[diesel(primary_key(id))]
#[diesel(table_name = parts)]
#[diesel(belongs_to(PartType, foreign_key = what))]
pub struct Part {
    /// The primary key
    pub id: PartId,
    /// The owner
    pub owner: UserId,
    /// The type of the part
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    /// purchase date
    #[serde_as(as = "Rfc3339")]
    pub purchase: OffsetDateTime,
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
    #[serde_as(as = "Rfc3339")]
    pub last_used: OffsetDateTime,
    /// Was it disposed? If yes, when?
    #[serde_as(as = "Option<Rfc3339>")]
    pub disposed_at: Option<OffsetDateTime>,
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Insertable)]
#[diesel(table_name = parts)]
pub struct NewPart {
    /// The owner
    pub owner: UserId,
    /// The type of the part
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    #[serde_as(as = "Option<Rfc3339>")]
    pub purchase: Option<OffsetDateTime>,
}

use serde_with::serde_as;
use time::format_description::well_known::Rfc3339;
#[serde_as]
#[derive(Clone, Debug, PartialEq, Deserialize, AsChangeset)]
#[diesel(table_name = parts)]
#[diesel(treat_none_as_null = true)]
pub struct ChangePart {
    pub id: PartId,
    /// The owner
    pub owner: UserId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    #[serde_as(as = "Rfc3339")]
    pub purchase: OffsetDateTime,
    /// Was it disposed? If yes, when?
    #[serde_as(as = "Option<Rfc3339>")]
    pub disposed_at: Option<OffsetDateTime>,
}

#[derive(DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartId(i32);

NewtypeDisplay! { () pub struct PartId(); }
NewtypeFrom! { () pub struct PartId(i32); }

impl PartId {
    pub fn new(id: i32) -> PartId {
        PartId(id)
    }

    pub fn get(id: i32, user: &dyn Person, conn: &mut AppConn) -> AnyResult<PartId> {
        PartId(id).checkuser(user, conn)
    }

    /// get the part with id part
    pub fn part(self, user: &dyn Person, conn: &mut AppConn) -> AnyResult<Part> {
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
    pub fn name(self, conn: &mut AppConn) -> AnyResult<String> {
        parts::table
            .find(self)
            .select(parts::name)
            .first(conn)
            .with_context(|| format!("part {} does not exist", self))
    }

    pub fn what(self, conn: &mut AppConn) -> AnyResult<PartTypeId> {
        parts::table
            .find(self)
            .select(parts::what)
            .first(conn)
            .with_context(|| format!("part {} does not exist", self))
    }

    /// check if the given user is the owner or an admin.
    /// Returns Forbidden if not.
    pub fn checkuser(self, user: &dyn Person, conn: &mut AppConn) -> AnyResult<PartId> {
        use schema::parts::dsl::*;

        if user.is_admin() {
            return Ok(self);
        }

        let own = parts
            .find(self)
            .filter(owner.eq(user.get_id()))
            .select(owner)
            .first::<UserId>(conn)?;
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
    pub fn apply_usage(
        self,
        usage: &Usage,
        start: OffsetDateTime,
        conn: &mut AppConn,
    ) -> AnyResult<Part> {
        use schema::parts::dsl::*;

        trace!("Applying usage {:?} to part {}", usage, self);

        Ok(conn.transaction(|conn| {
            let part: Part = parts.find(self).for_update().get_result(conn)?;
            diesel::update(parts.find(self))
                .set((
                    time.eq(time + usage.time),
                    climb.eq(climb + usage.climb),
                    descend.eq(descend + usage.descend),
                    distance.eq(distance + usage.distance),
                    count.eq(count + usage.count),
                    purchase.eq(min(part.purchase, start)),
                    last_used.eq(max(part.last_used, start)),
                ))
                .get_result::<Part>(conn)
        })?)
    }
}

impl Part {
    pub fn get_all(user: &dyn Person, conn: &mut AppConn) -> AnyResult<Vec<Part>> {
        use schema::parts::dsl::*;

        Ok(parts
            .filter(owner.eq(user.get_id()))
            .order_by(last_used)
            .load::<Part>(conn)?)
    }

    /// reset all usage counters for all parts of a person
    ///
    /// returns the list of main gears affected
    pub fn reset(user: &dyn Person, conn: &mut AppConn) -> AnyResult<Vec<PartId>> {
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
                last_used.eq(purchase),
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
    pub fn create(self, user: &dyn Person, conn: &mut AppConn) -> AnyResult<Part> {
        use schema::parts::dsl::*;
        info!("Create {:?}", self);

        user.check_owner(
            self.owner,
            format!("user {} cannot create this part", user.get_id()),
        )?;

        let now = OffsetDateTime::now_utc();
        let values = (
            owner.eq(self.owner),
            what.eq(self.what),
            name.eq(self.name),
            vendor.eq(self.vendor),
            model.eq(self.model),
            purchase.eq(self.purchase.unwrap_or(now)),
            last_used.eq(self.purchase.unwrap_or(now)),
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
    pub fn change(&self, user: &dyn Person, conn: &mut AppConn) -> AnyResult<Part> {
        use schema::parts::dsl::*;
        info!("Change {:?}", self);

        user.check_owner(
            self.owner,
            format!("user {} cannot create this part", user.get_id()),
        )?;

        let part: Part = diesel::update(parts.filter(id.eq(self.id)))
            .set(self)
            .get_result(conn)?;
        Ok(part)
    }
}
