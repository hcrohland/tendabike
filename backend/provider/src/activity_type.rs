//! The common activity-type table (spec #838), owned by the provider kit:
//! the wire-name → [`ActTypeId`] arms both providers map identically. Each
//! provider calls it first and falls back to its own cycling-subtype arms
//! (provider-specific, so they stay there).

use tb_domain::ActTypeId;

/// Maps a wire activity-type name to its [`ActTypeId`], for the arms shared
/// by both providers.
///
/// `None` when the name is not in the common table — the provider falls back
/// to its own cycling-subtype arms before deciding the name is unsupported.
pub fn common_type(t: &str) -> Option<ActTypeId> {
    Some(
        match t {
            // ── cycling ───────────────────────────────────────────────────────
            "Ride" => 1,
            "VirtualRide" => 5,
            "EBikeRide" => 9,
            // ── everything else (shared non-cycling types) ───────────────────
            "Snowboard" => 2,
            "Run" | "VirtualRun" => 3,
            "Hike" => 4,
            "AlpineSki" => 6,
            "Walk" => 8,
            "BackcountrySki" => 10,
            "Workout" => 0,
            "StandUpPaddling" => 0,
            "Windsurf" => 0,
            "Kitesurf" => 0,
            "Rowing" => 0,
            "WaterSport" => 0,
            "RockClimbing" => 0,
            "Handcycle" => 0,
            "Canoeing" => 0,
            "Crossfit" => 0,
            "Elliptical" => 0,
            "Golf" => 0,
            "IceSkate" => 0,
            "InlineSkate" => 0,
            "Kayaking" => 0,
            "NordicSki" => 0,
            "RollerSki" => 0,
            "Sail" => 0,
            "Skateboard" => 0,
            "Snowshoe" => 0,
            "Soccer" => 0,
            "StairStepper" => 0,
            "Surfing" => 0,
            "Swim" => 0,
            "Velomobile" => 0,
            "WeightTraining" => 0,
            "Wheelchair" => 0,
            "Yoga" => 0,
            _ => return None,
        }
        .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_the_cycling_arms() {
        assert_eq!(common_type("Ride"), Some(ActTypeId::from(1)));
        assert_eq!(common_type("VirtualRide"), Some(ActTypeId::from(5)));
        assert_eq!(common_type("EBikeRide"), Some(ActTypeId::from(9)));
    }

    #[test]
    fn maps_the_shared_non_cycling_arms() {
        assert_eq!(common_type("Snowboard"), Some(ActTypeId::from(2)));
        assert_eq!(common_type("Run"), Some(ActTypeId::from(3)));
        assert_eq!(common_type("VirtualRun"), Some(ActTypeId::from(3)));
        assert_eq!(common_type("Hike"), Some(ActTypeId::from(4)));
        assert_eq!(common_type("AlpineSki"), Some(ActTypeId::from(6)));
        assert_eq!(common_type("Walk"), Some(ActTypeId::from(8)));
        assert_eq!(common_type("BackcountrySki"), Some(ActTypeId::from(10)));
    }

    #[test]
    fn maps_the_zero_arms() {
        for name in [
            "Workout",
            "Handcycle",
            "NordicSki",
            "RollerSki",
            "Velomobile",
            "Swim",
            "Yoga",
        ] {
            assert_eq!(common_type(name), Some(ActTypeId::from(0)));
        }
    }

    #[test]
    fn unknown_and_icu_only_names_are_none() {
        for name in [
            "MountainBikeRide",
            "GravelRide",
            "Cyclocross",
            "TrackRide",
            "Bogus",
        ] {
            assert_eq!(common_type(name), None);
        }
    }
}
