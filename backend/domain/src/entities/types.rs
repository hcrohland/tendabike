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

//! This module defines the types used in the tendabike application.
//!
//! It includes the types for parts and activities, as well as their relationships.
//!
//! The types defined in this module are used throughout the application to ensure type safety and consistency.

use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};

use crate::*;

mod objects;
use objects::{ACTTYPES, PARTTYPES};

#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize)]
pub struct PartTypeId(i32);

NewtypeDisplay! { () pub struct PartTypeId(); }
NewtypeFrom! { () pub struct PartTypeId(i32); }

/// List of of all valid part types.
///
/// We distingish main parts from spares:
/// - Main parts can be used for an activity - like a bike
/// - Spares can be attached to other parts and are subparts of main parts
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PartType {
    /// The primary key
    pub id: PartTypeId,
    /// The display name
    pub name: String,
    /// To which main gear type this part belongs
    pub main: PartTypeId,
    /// Part types that can be attached
    pub hooks: Vec<PartTypeId>,
    /// the order for displaying types
    pub order: i32,
    /// Potential group
    pub group: Option<String>,
}

impl PartType {
    pub fn all_ordered() -> Vec<Self> {
        PARTTYPES.values().cloned().collect()
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize)]
pub struct ActTypeId(i32);

NewtypeDisplay! { () pub struct ActTypeId(); }
NewtypeFrom! { () pub struct ActTypeId(i32); }

/// The list of activity types
/// Includes the kind of gear which can be used for this activity
/// multiple gears are possible
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityType {
    /// The primary key
    pub id: ActTypeId,
    /// The name
    pub name: String,
    /// Gears which can be used for this activity type
    pub gear_type: PartTypeId,
}

impl PartTypeId {
    /// get the full type for a type_id
    pub(crate) fn get(self) -> TbResult<PartType> {
        PARTTYPES
            .get(&self)
            .cloned()
            .ok_or(crate::Error::NotFound(format!(
                "parttype {self} does not exist"
            )))
    }

    pub(crate) fn is_main(self) -> TbResult<bool> {
        let t = self.get()?;
        Ok(t.hooks.is_empty())
    }

    /// recursively look for subtypes to self in the PartType vector
    fn filter_types(self, types: &mut Vec<PartType>) -> Vec<PartType> {
        // let mut res = types
        //     .drain_filter(|x| x.hooks.contains(&self) || x.id == self)
        //     .collect::<Vec<_>>();
        let mut res = Vec::new();
        let mut i = 0;
        while i < types.len() {
            let x = &types[i];
            if x.hooks.contains(&self) || x.id == self {
                res.push(types.remove(i));
            } else {
                i += 1;
            }
        }

        for t in res.clone().iter() {
            res.append(&mut t.id.filter_types(types));
        }
        res
    }

    /// get all the type_ids you can attach - even indirectly - to this type_id
    pub(crate) fn subtypes(self) -> Vec<PartTypeId> {
        let mut types = PartType::all_ordered();
        self.filter_types(&mut types)
            .into_iter()
            .map(|t| t.id)
            .collect()
    }

    /// Get the activity types valid for this part_type
    pub(crate) fn act_types(&self) -> Vec<ActTypeId> {
        ACTTYPES
            .values()
            .filter(|a| a.gear_type == *self)
            .map(|a| a.id)
            .collect()
    }
}

impl ActivityType {
    pub fn all_ordered() -> Vec<ActivityType> {
        ACTTYPES.values().cloned().collect()
    }
}
