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

use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};

use crate::*;

mod objects;
use objects::{ACTTYPES, PARTTYPES};

#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    From,
    Into,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Serialize,
    Deserialize,
)]
pub struct PartTypeId(i32);

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

#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    From,
    Into,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Serialize,
    Deserialize,
)]
pub struct ActTypeId(i32);

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
    /// Create a new PartTypeId from an i32 value.
    pub const fn from_id(id: i32) -> Self {
        Self(id)
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn part_type_all_ordered_nonempty() {
        let types = PartType::all_ordered();
        assert_eq!(types.len(), 29);
    }

    #[test]
    fn part_type_all_ordered_sorted_by_id() {
        let types = PartType::all_ordered();
        for w in types.windows(2) {
            assert!(w[0].id < w[1].id, "expected {} < {}", w[0].id, w[1].id);
        }
    }

    #[test]
    fn parttypeid_get_valid() {
        let bike = PartTypeId::from_id(1).get().unwrap();
        assert_eq!(bike.name, "Bike");
        assert_eq!(bike.id, PartTypeId::from_id(1));
    }

    #[test]
    fn parttypeid_get_invalid() {
        let err = PartTypeId::from_id(999).get().unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
    }

    #[test]
    fn parttypeid_is_main_bike() {
        assert!(PartTypeId::from_id(1).is_main().unwrap());
    }

    #[test]
    fn parttypeid_is_main_shoe() {
        assert!(PartTypeId::from_id(301).is_main().unwrap());
    }

    #[test]
    fn parttypeid_is_main_chain() {
        assert!(!PartTypeId::from_id(4).is_main().unwrap());
    }

    #[test]
    fn parttypeid_is_main_invalid() {
        assert!(PartTypeId::from_id(999).is_main().is_err());
    }

    #[test]
    fn parttypeid_subtypes_bike() {
        let subs = PartTypeId::from_id(1).subtypes();
        assert!(subs.contains(&PartTypeId::from_id(4)));
        assert!(subs.contains(&PartTypeId::from_id(3)));
        assert!(subs.contains(&PartTypeId::from_id(2)));
        assert!(subs.contains(&PartTypeId::from_id(11)));
        assert!(subs.contains(&PartTypeId::from_id(14)));
        assert!(subs.contains(&PartTypeId::from_id(1)));
    }

    #[test]
    fn parttypeid_subtypes_tire_only_self() {
        let subs = PartTypeId::from_id(3).subtypes();
        assert_eq!(subs, vec![PartTypeId::from_id(3)]);
    }

    #[test]
    fn parttypeid_subtypes_shoe_includes_binding() {
        let subs = PartTypeId::from_id(302).subtypes();
        assert!(subs.contains(&PartTypeId::from_id(309)));
    }

    #[test]
    fn parttypeid_act_types_bike() {
        let acts = PartTypeId::from_id(1).act_types();
        assert_eq!(
            acts,
            vec![ActTypeId::from(1), ActTypeId::from(5), ActTypeId::from(9)]
        );
    }

    #[test]
    fn parttypeid_act_types_shoe() {
        let acts = PartTypeId::from_id(301).act_types();
        assert_eq!(
            acts,
            vec![ActTypeId::from(3), ActTypeId::from(4), ActTypeId::from(8)]
        );
    }

    #[test]
    fn activitytype_all_ordered_nonempty() {
        let acts = ActivityType::all_ordered();
        assert_eq!(acts.len(), 11);
    }
}
