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

//! This module contains the domain logic for the `User` entity.
//!
//! The `User` entity represents a user of the `tendabike` application.
//! It contains information such as the user's name, whether they are an admin, and their activity and parts statistics.
//!
//! The `UserId` type is a newtype wrapper around an `i32` and is used to represent the unique identifier of a `User`.
//!
//! The `Stat` struct is used to represent the statistics of a `User`, including the number of parts and activities associated with the user.
//!
//! The `User` struct contains the fields of a user, including their `id`, `name`, `firstname`, and `is_admin` status.
//!
//! The `Person` trait is implemented for the `User` struct and provides methods for getting the user's `id` and `is_admin` status.
//!
//! The `create`, `update`, `read`, and `get_stat` methods are implemented for the `UserId` type and provide CRUD functionality for `User` entities.

use super::*;
use schema::users;

#[derive(
    DieselNewType, Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct UserId(i32);
NewtypeDisplay! { () pub struct UserId(); }
NewtypeFrom! { () pub struct UserId(i32); }

#[derive(Clone, Debug, Queryable, Insertable, Serialize, Deserialize)]
pub struct User {
    id: UserId,
    name: String,
    firstname: String,
    is_admin: bool,
}

#[derive(Debug, Serialize)]
pub struct Stat {
    pub user: User,
    parts: i64,
    activities: i64,
}

impl UserId {
    pub async fn read(self, conn: &mut AppConn) -> AnyResult<User> {
        Ok(users::table.find(self).get_result(conn).await?)
    }

    pub async fn get_stat(self, conn: &mut AppConn) -> AnyResult<Stat> {
        let user = self.read(conn).await?;
        let parts = {
            use schema::parts::dsl::{owner, parts};
            parts.count().filter(owner.eq(self)).first(conn).await?
        };
        let activities = {
            use schema::activities::dsl::*;
            activities.count().filter(user_id.eq(self)).first(conn).await?
        };
        Ok(Stat {
            user,
            parts,
            activities,
        })
    }

    pub async fn create(firstname_: &str, lastname: &str, conn: &mut AppConn) -> AnyResult<Self> {
        use crate::schema::users::dsl::*;

        let user: User = diesel::insert_into(users)
            .values((
                firstname.eq(firstname_),
                name.eq(lastname),
                is_admin.eq(false),
            ))
            .get_result(conn)
            .await
            .context("Could not create user")?;
        Ok(user.id)
    }

    pub async fn update(
        &self,
        firstname_: &str,
        lastname: &str,
        conn: &mut AppConn,
    ) -> AnyResult<Self> {
        use crate::schema::users::dsl::*;

        let user: User = diesel::update(users.filter(id.eq(self)))
            .set((firstname.eq(firstname_), name.eq(lastname)))
            .get_result(conn)
            .await
            .context("Could not update user")?;
        Ok(user.id)
    }

    pub async fn is_admin(&self, conn: &mut AppConn) -> AnyResult<bool> {
        self.read(conn).await.map(|u| u.is_admin)
    }
}

impl Person for User {
    fn get_id(&self) -> UserId {
        self.id
    }
    fn is_admin(&self) -> bool {
        self.is_admin
    }
}
