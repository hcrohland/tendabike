use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct PartTypeId(i32);

NewtypeDisplay! { () pub struct PartTypeId(); }
NewtypeFrom! { () pub struct PartTypeId(i32); }

/// List of of all valid part types.
///
/// We distingish main parts from spares:
/// - Main parts can be used for an activity - like a bike
/// - Spares can be attached to other parts and are subparts of main parts
#[derive(Clone, Debug, PartialEq)]
pub struct PartType {
    /// The primary key
    pub id: PartTypeId,
    /// The display name
    pub name: String,
    /// is it a main part? I.e. can it be used for an activity?
    pub main: PartTypeId,
    /// Part types that can be attached
    pub hooks: Vec<PartTypeId>,
    /// the order for displaying types
    pub order: i32,
    /// Potential group
    pub group: Option<String>
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ActTypeId(i32);

NewtypeDisplay! { () pub struct ActTypeId(); }
NewtypeFrom! { () pub struct ActTypeId(i32); }


lazy_static! {
    static ref ACTIVITY_TYPES: HashMap<ActTypeId, ActivityType> = {
        let mut m = HashMap::new();
        let types =
        [
            (1,"Bike Ride",1),
            (2,"Snowboard",302),
            (3,"Running",301),
            (4,"Hiking",301),
            (5,"Virtual Ride",1),
            (6,"Skiing",303),
            (8,"Walk",301),
            (7,"Splitboard Tour",302),
            (10,"Skitour",303),
            (0,"Whatever",304),
            (9,"EBike Ride",1),
        ];
        for (id, name, gear_type) in types {
            let a = 
                ActivityType {
                    id: id.into(), 
                    name: name.into(), 
                    gear_type: gear_type.into()
                };
            m.insert(a.id, a);
        }
        m
    };
}

lazy_static! {
    static ref PART_TYPES: HashMap<PartTypeId, PartType> = {
        let mut m = HashMap::new();
        let types =
        [
            (303,"Ski",303,vec![],9999,None),
            (301,"Shoe",301,vec![],9999,None),
            (304,"Whatever",304,vec![],9999,None),
            (302,"Snowboard",302,vec![],9999,None),
            (305,"SUP board",305,vec![],9999,None),
            (306,"Windsurf Board",306,vec![],9999,None),
            (307,"Kite Board",307,vec![],9999,None),
            (308,"Rowing boat",308,vec![],9999,None),
            (309,"binding",302,vec![302],9999,None),
            (11,"saddle",1,vec![10],17,None),
            (1,"Bike",1,vec![],1,None),
            (18,"pedal",1,vec![13],10,None),
            (19,"bottom bracket",1,vec![1],13,None),
            (3,"tire",1,vec![2,5],7,Some("Tires")),
            (2,"front wheel",1,vec![1],5,Some("Wheels")),
            (5,"rear wheel",1,vec![1],6,Some("Wheels")),
            (12,"derailleur",1,vec![1],12,Some("Drive train")),
            (14,"chainring",1,vec![13],9,Some("Drive train")),
            (4,"chain",1,vec![1],11,Some("Drive train")),
            (13,"crank",1,vec![1],13,Some("Drive train")),
            (9,"cassette",1,vec![5],10,Some("Drive train")),
            (6,"brake pad",1,vec![7,8],4,Some("Brakes")),
            (7,"front brake",1,vec![1],2,Some("Brakes")),
            (8,"rear brake",1,vec![1],3,Some("Brakes")),
            (15,"brake rotor",1,vec![2,5],8,Some("Brakes")),
            (17,"rear shock",1,vec![1],15,Some("Shock")),
            (16,"fork",1,vec![1],14,Some("Fork")),
            (10,"seat post",1,vec![1],16,Some("Seat post"   )),
            ];
        for (id,name,main,hooks,order,group) in types {
            let a = 
                PartType {
                    id: id.into(), 
                    name: name.into(), 
                    main: main.into(),
                    hooks: hooks.into_iter().map(|a|a.into()).collect(),
                    order,
                    group: group.map(|a|a.into())
                };
            m.insert(a.id, a);
        }
        m
    };
}



/// The list of activity types
/// Includes the kind of gear which can be used for this activity
/// multiple gears are possible
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityType {
    /// The primary key
    pub id: ActTypeId,
    /// The name
    pub name: String,
    /// Gears which can be used for this activity type
    pub gear_type: PartTypeId,
}

impl ActTypeId {
    pub fn get(&self) -> Option<&'static ActivityType> {
        ACTIVITY_TYPES.get(self)
    }
}

impl PartTypeId {

    /// Get the activity types valid for this part_type
    pub fn act_types(&self) -> Vec<ActTypeId> {
        ACTIVITY_TYPES
            .values()
            .filter(|x| x.gear_type != *self)
            .map (|x| x.id)
            .collect()
    }

    /// recursively look for subtypes to self in the PartType vector
    fn filter_types(self, types: &mut Vec<&'static PartType>) -> Vec<&'static PartType> {
        let mut res = types
            .drain_filter(|x| x.hooks.contains(&self) || x.id == self)
            .collect::<Vec<_>>();
        for t in res.clone().iter() {
            res.append(&mut t.id.filter_types(types));
        }
        res
    }

    /// get the full type for a type_id
    pub fn get (&self) -> Option<&'static PartType> {
        PART_TYPES.get(self)
    }

    /// get all the types you can attach - even indirectly - to this type_id
    pub fn subtypes(self) -> Vec<&'static PartType> {
        let mut types = PART_TYPES.values().collect();
        self.filter_types(&mut types)
    }
}
