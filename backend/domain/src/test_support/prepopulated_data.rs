//! Prepopulated store data for tests.
//! Generated from `build_snapshot_json` test.
//! Regenerate by running: cargo test -p tb_domain build_snapshot_json -- --nocapture

/// Formatted JSON snapshot of the prepopulated store.
/// This is a pretty-printed JSON string that gets deserialized by MemStore::prepopulated().
pub const SNAPSHOT_JSON: &str = r#"{
  "parts": [
    {
      "id": 7,
      "owner": 1,
      "what": 1,
      "name": "Road Bike",
      "vendor": "Trek",
      "model": "Domane",
      "purchase": "2022-11-14T22:00:00Z",
      "last_used": "2022-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-0819b7fc2292",
      "source": null,
      "notes": "Road bike frame",
      "shop": null
    },
    {
      "id": 15,
      "owner": 1,
      "what": 3,
      "name": "Spare Tire",
      "vendor": "Continental",
      "model": "Supersonic",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-08f4c5ea4908",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 9,
      "owner": 1,
      "what": 5,
      "name": "Rear Wheel B",
      "vendor": "Mavic",
      "model": "Carbon WS",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-083bcbe6e135",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 3,
      "owner": 1,
      "what": 5,
      "name": "Rear Wheel A",
      "vendor": "DT Swiss",
      "model": "XR 1501",
      "purchase": "2023-05-18T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-07793fb4b3ba",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 16,
      "owner": 1,
      "what": 2,
      "name": "Spare Wheel",
      "vendor": "HED",
      "model": "Stinger 3",
      "purchase": "2023-05-18T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-090f122b3304",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 5,
      "owner": 1,
      "what": 3,
      "name": "Tire Front A",
      "vendor": "Continental",
      "model": "GP5000",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-0798daacfbd5",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 2,
      "owner": 1,
      "what": 2,
      "name": "Front Wheel A",
      "vendor": "Zipp",
      "model": "404 Firecrest",
      "purchase": "2023-05-18T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-076a33e8d963",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 1,
      "owner": 1,
      "what": 1,
      "name": "Main Bike",
      "vendor": "TendaBike",
      "model": "Standard Frame",
      "purchase": "2022-11-14T22:00:00Z",
      "last_used": "2023-05-19T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-075caa3c6692",
      "source": null,
      "notes": "Main bike frame",
      "shop": null
    },
    {
      "id": 6,
      "owner": 1,
      "what": 3,
      "name": "Tire Rear A",
      "vendor": "Continental",
      "model": "GP5000 ST",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-07a43f6ba52f",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 4,
      "owner": 1,
      "what": 4,
      "name": "Chain A",
      "vendor": "Shimano",
      "model": "CN-M510",
      "purchase": "2023-05-18T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-07857ca515c6",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 10,
      "owner": 1,
      "what": 4,
      "name": "Chain B",
      "vendor": "SRAM",
      "model": "PC-XX1",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-0848842fe87e",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 11,
      "owner": 1,
      "what": 3,
      "name": "Tire Front B",
      "vendor": "Schwalbe",
      "model": "One",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-085afad47308",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 13,
      "owner": 1,
      "what": 4,
      "name": "Spare Chain 1",
      "vendor": "Shimano",
      "model": "HG-54",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-08d7b7ef51ed",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 8,
      "owner": 1,
      "what": 2,
      "name": "Front Wheel B",
      "vendor": "Mavic",
      "model": "Carbon WS",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-0826a936158c",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 12,
      "owner": 1,
      "what": 3,
      "name": "Tire Rear B",
      "vendor": "Schwalbe",
      "model": "One Plus",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-0861bd6b46d0",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 14,
      "owner": 1,
      "what": 4,
      "name": "Spare Chain 2",
      "vendor": "SRAM",
      "model": "PC-1031",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-08e70ff6253a",
      "source": null,
      "notes": "Test part",
      "shop": null
    },
    {
      "id": 17,
      "owner": 1,
      "what": 3,
      "name": "Spare Wheel Tire",
      "vendor": "Continental",
      "model": "GP5000",
      "purchase": "2023-11-14T22:00:00Z",
      "last_used": "2023-11-14T22:00:00Z",
      "disposed_at": null,
      "usage": "01a04c37-93cd-7872-9a0d-091cd3840a27",
      "source": null,
      "notes": "Test part",
      "shop": null
    }
  ],
  "attachments": [
    {
      "part_id": 6,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 3,
      "hook": 3,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-0800adac7ff3"
    },
    {
      "part_id": 12,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 9,
      "hook": 3,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-08c9ff94c204"
    },
    {
      "part_id": 9,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 7,
      "hook": 1,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-08844ea80933"
    },
    {
      "part_id": 16,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 1,
      "hook": 1,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-09284075fa42"
    },
    {
      "part_id": 12,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 8,
      "hook": 3,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-08b32edca80c"
    },
    {
      "part_id": 17,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 16,
      "hook": 3,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-0931173acf98"
    },
    {
      "part_id": 4,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 1,
      "hook": 1,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-07d06152f99c"
    },
    {
      "part_id": 10,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 7,
      "hook": 1,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-089634179fbb"
    },
    {
      "part_id": 8,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 7,
      "hook": 1,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-087efdbd0a70"
    },
    {
      "part_id": 11,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 8,
      "hook": 3,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-08ac3bf95d96"
    },
    {
      "part_id": 3,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 1,
      "hook": 1,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-07c19820333d"
    },
    {
      "part_id": 2,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 1,
      "hook": 1,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-07b244c887aa"
    },
    {
      "part_id": 6,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 2,
      "hook": 3,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-07f2cd7be2e8"
    },
    {
      "part_id": 5,
      "attached": "2023-01-01T00:00:00Z",
      "gear": 2,
      "hook": 3,
      "detached": "9100-01-01T00:00:00Z",
      "usage": "01a04c37-93cd-7872-9a0d-07e621b1e390"
    }
  ],
  "usages": [
    {
      "id": "01a04c37-93cd-7872-9a0d-07d06152f99c",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    },
    {
      "id": "01a04c37-93cd-7872-9a0d-07c19820333d",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    },
    {
      "id": "01a04c37-93cd-7872-9a0d-07b244c887aa",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    },
    {
      "id": "01a04c37-93cd-7872-9a0d-07793fb4b3ba",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    },
    {
      "id": "01a04c37-93cd-7872-9a0d-076a33e8d963",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    },
    {
      "id": "01a04c37-93cd-7872-9a0d-090f122b3304",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    },
    {
      "id": "01a04c37-93cd-7872-9a0d-09284075fa42",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    },
    {
      "id": "01a04c37-93cd-7872-9a0d-075caa3c6692",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    },
    {
      "id": "01a04c37-93cd-7872-9a0d-07857ca515c6",
      "time": 8025,
      "distance": 125000,
      "climb": 1100,
      "descend": 1100,
      "energy": 1500,
      "count": 3
    }
  ],
  "activities": [
    {
      "id": 1,
      "user_id": 1,
      "what": 1,
      "name": "Morning Ride",
      "start": "2023-05-18T22:13:20Z",
      "duration": 3600,
      "time": 25,
      "distance": 50000,
      "climb": 400,
      "descend": null,
      "energy": 500,
      "gear": 1,
      "device_name": null,
      "external_id": null
    },
    {
      "id": 1,
      "user_id": 1,
      "what": 1,
      "name": "Hill Repeats",
      "start": "2023-05-19T00:13:20Z",
      "duration": 1800,
      "time": 5200,
      "distance": 40000,
      "climb": 600,
      "descend": null,
      "energy": 500,
      "gear": 1,
      "device_name": null,
      "external_id": null
    },
    {
      "id": 1,
      "user_id": 1,
      "what": 1,
      "name": "Recovery Spin",
      "start": "2023-05-19T22:13:20Z",
      "duration": 2400,
      "time": 2800,
      "distance": 35000,
      "climb": 100,
      "descend": null,
      "energy": 500,
      "gear": 1,
      "device_name": null,
      "external_id": null
    }
  ]
}"#;
