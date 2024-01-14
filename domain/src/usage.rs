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

//! This module contains the `Usage` struct and its implementation.
//! The `Usage` struct represents the usage of a part, including time, distance, climbing, descending, power, and count.
//! It also provides methods to add an activity to the usage.

use std::ops::{Add, Neg};

#[derive(Debug)]
pub struct Usage {
    // usage time
    pub time: i32,
    /// Usage distance
    pub distance: i32,
    /// Overall climbing
    pub climb: i32,
    /// Overall descending
    pub descend: i32,
    /// Overall descending
    pub power: i32,
    /// number of activities
    pub count: i32,
}

impl Default for Usage {
    fn default() -> Usage {
        Usage {
            time: 0,
            climb: 0,
            descend: 0,
            power: 0,
            distance: 0,
            count: 0,
        }
    }
}

impl Add for Usage {
    type Output = Self;
    /// Add an activity to of a usage
    ///
    /// If the descend value is missing, assume descend = climb
    fn add(self, act: Self) -> Self {
        Usage {
            time: self.time + act.time,
            climb: self.climb + act.climb,
            descend: self.descend + act.descend,
            power: self.power + act.power,
            distance: self.distance + act.distance,
            count: self.count + act.count,
        }
    }
}

impl Neg for Usage {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Usage {
            time: -self.time,
            climb: -self.climb,
            descend: -self.descend,
            power: -self.power,
            distance: -self.distance,
            count: -self.count,
        }
    }
}
