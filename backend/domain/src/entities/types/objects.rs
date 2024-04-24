use std::collections::BTreeMap;

use lazy_static::lazy_static;

use crate::{ActTypeId, ActivityType, PartType, PartTypeId};

lazy_static! {
    pub(super) static ref ACTTYPES: BTreeMap<ActTypeId, ActivityType> = {
        let mut m = BTreeMap::new();
        let acttypes: Vec<ActivityType> = serde_json::from_str(ACTLIST).expect("acttypes failure");
        for t in acttypes {
            m.insert(t.id, t);
        }
        m
    };
    pub(super) static ref PARTTYPES: BTreeMap<PartTypeId, PartType> = {
        let mut m = BTreeMap::new();
        let types: Vec<PartType> = serde_json::from_str(PARTLIST).expect("parttypes failure");
        for t in types {
            m.insert(t.id, t);
        }
        m
    };
}

const ACTLIST: &str = r#"
[
  {
    "id": 0,
    "name": "Whatever",
    "gear_type": 304
  },
  {
    "id": 1,
    "name": "Bike Ride",
    "gear_type": 1
  },
  {
    "id": 2,
    "name": "Snowboard",
    "gear_type": 302
  },
  {
    "id": 3,
    "name": "Running",
    "gear_type": 301
  },
  {
    "id": 4,
    "name": "Hiking",
    "gear_type": 301
  },
  {
    "id": 5,
    "name": "Virtual Ride",
    "gear_type": 1
  },
  {
    "id": 6,
    "name": "Skiing",
    "gear_type": 303
  },
  {
    "id": 7,
    "name": "Splitboard Tour",
    "gear_type": 302
  },
  {
    "id": 8,
    "name": "Walk",
    "gear_type": 301
  },
  {
    "id": 9,
    "name": "EBike Ride",
    "gear_type": 1
  },
  {
    "id": 10,
    "name": "Skitour",
    "gear_type": 303
  }
]
"#;

const PARTLIST: &str = r#"
[
  {
    "id": 1,
    "name": "Bike",
    "main": 1,
    "hooks": [],
    "order": 1,
    "group": null
  },
  {
    "id": 2,
    "name": "front wheel",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 5,
    "group": "Wheels"
  },
  {
    "id": 3,
    "name": "tire",
    "main": 1,
    "hooks": [
      2,
      5
    ],
    "order": 7,
    "group": "Tires"
  },
  {
    "id": 4,
    "name": "chain",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 11,
    "group": "Drive train"
  },
  {
    "id": 5,
    "name": "rear wheel",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 6,
    "group": "Wheels"
  },
  {
    "id": 6,
    "name": "brake pad",
    "main": 1,
    "hooks": [
      7,
      8
    ],
    "order": 4,
    "group": "Brakes"
  },
  {
    "id": 7,
    "name": "front brake",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 2,
    "group": "Brakes"
  },
  {
    "id": 8,
    "name": "rear brake",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 3,
    "group": "Brakes"
  },
  {
    "id": 9,
    "name": "cassette",
    "main": 1,
    "hooks": [
      5
    ],
    "order": 10,
    "group": "Drive train"
  },
  {
    "id": 10,
    "name": "seat post",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 18,
    "group": "Seat post"
  },
  {
    "id": 11,
    "name": "saddle",
    "main": 1,
    "hooks": [
      10
    ],
    "order": 19,
    "group": null
  },
  {
    "id": 12,
    "name": "derailleur",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 12,
    "group": "Drive train"
  },
  {
    "id": 13,
    "name": "crank",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 13,
    "group": "Drive train"
  },
  {
    "id": 14,
    "name": "chainring",
    "main": 1,
    "hooks": [
      13
    ],
    "order": 9,
    "group": "Drive train"
  },
  {
    "id": 15,
    "name": "brake rotor",
    "main": 1,
    "hooks": [
      2,
      5
    ],
    "order": 8,
    "group": "Brakes"
  },
  {
    "id": 16,
    "name": "fork",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 16,
    "group": "Fork"
  },
  {
    "id": 17,
    "name": "rear shock",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 15,
    "group": "Shock"
  },
  {
    "id": 18,
    "name": "pedal",
    "main": 1,
    "hooks": [
      13
    ],
    "order": 10,
    "group": null
  },
  {
    "id": 19,
    "name": "bottom bracket",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 14,
    "group": null
  },
  {
    "id": 20,
    "name": "headset",
    "main": 1,
    "hooks": [
      1
    ],
    "order": 17,
    "group": null
  },
  {
    "id": 301,
    "name": "Shoe",
    "main": 301,
    "hooks": [],
    "order": 9999,
    "group": null
  },
  {
    "id": 302,
    "name": "Snowboard",
    "main": 302,
    "hooks": [],
    "order": 9999,
    "group": null
  },
  {
    "id": 303,
    "name": "Ski",
    "main": 303,
    "hooks": [],
    "order": 9999,
    "group": null
  },
  {
    "id": 304,
    "name": "Whatever",
    "main": 304,
    "hooks": [],
    "order": 9999,
    "group": null
  },
  {
    "id": 305,
    "name": "SUP board",
    "main": 305,
    "hooks": [],
    "order": 9999,
    "group": null
  },
  {
    "id": 306,
    "name": "Windsurf Board",
    "main": 306,
    "hooks": [],
    "order": 9999,
    "group": null
  },
  {
    "id": 307,
    "name": "Kite Board",
    "main": 307,
    "hooks": [],
    "order": 9999,
    "group": null
  },
  {
    "id": 308,
    "name": "Rowing boat",
    "main": 308,
    "hooks": [],
    "order": 9999,
    "group": null
  },
  {
    "id": 309,
    "name": "binding",
    "main": 302,
    "hooks": [
      302
    ],
    "order": 9999,
    "group": null
  }
]
"#;
