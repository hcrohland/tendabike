use super::*;
#[derive(Serialize, Debug, Default)]
pub struct Summary {
    pub activities: Vec<Activity>,
    pub parts: Vec<Part>,
    pub attachments: Vec<AttachmentDetail>
}

impl Summary {
pub fn new(activities: Vec<activity::Activity>, parts: Vec<Part>, attachments: Vec<AttachmentDetail>) -> Self {
        Summary {activities,parts,attachments}
}

pub fn append(&mut self, new: &mut Self) {
    self.activities.append(&mut new.activities);
    self.parts.append(&mut new.parts);
    self.attachments.append(&mut new.attachments);
}

pub fn merge(self, new: Summary) -> Summary {
    let mut hash = SumHash::new(self);
    hash.merge(new);
    hash.collect()
}

pub fn first(&self) -> ActivityId {
    self.activities[0].id
}
}

#[derive(Debug, Default)]
pub struct SumHash {
activities: HashMap<ActivityId, Activity>,
parts: HashMap<PartId, Part>,
atts: HashMap<String, AttachmentDetail>,
}

impl SumHash {
pub fn new(sum: Summary) -> Self {
    let mut hash = SumHash::default();
    hash.merge(sum);
    hash
}

pub fn merge(&mut self, ps: Summary)  {
    for act in ps.activities {
        self.activities.insert(act.id, act);
    }
    for part in ps.parts {
        self.parts.insert(part.id, part);
    }
    for att in ps.attachments {
        self.atts.insert(att.idx(), att);
    }
}

pub fn collect(self) -> Summary {
    Summary {
        activities: self.activities.into_iter().map(|(_,v)| v).collect(),
        parts: self.parts.into_iter().map(|(_,v)| v).collect(),
        attachments: self.atts.into_iter().map(|(_,v)| v).collect(),
    }
}
}
