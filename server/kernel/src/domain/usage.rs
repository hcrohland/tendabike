use super::*;

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

#[derive(Clone, Copy, PartialEq)]
pub enum Factor {
Add = 1,
Sub = -1,
No = 0,
}

impl Usage {
pub fn none() -> Usage {
    Usage {
        time: 0,
        climb: 0,
        descend: 0,
        power: 0,
        distance: 0,
        count: 0,
    }
}

/// Add an activity to of a usage
///
/// If the descend value is missing, assume descend = climb
pub fn add_activity(self, act: &Activity, factor: Factor) -> Usage {
    let factor = factor as i32;
    Usage {
        time: self.time + act.time.unwrap_or(0) * factor,
        climb: self.climb + act.climb.unwrap_or(0) * factor,
        descend: self.descend + act.descend.unwrap_or_else(|| act.climb.unwrap_or(0)) * factor,
        power: self.power + act.power.unwrap_or(0) * factor,
        distance: self.distance + act.distance.unwrap_or(0) * factor,
        count: self.count + factor,
    }
}
}
