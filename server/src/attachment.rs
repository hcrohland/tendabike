//! The list of all historical attachments. This is the central piece of TendaBike.
//!
//! Attachments can be hierarchical
//! They are identified by part_id and attached time

use chrono::MAX_DATETIME;
use rocket_contrib::json::Json;
use diesel::{self, QueryDsl, RunQueryDsl};

use crate::*;
use schema::attachments;


#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Event {
    /// the sub-part, which is attached to the hook
    part_id: PartId,
    /// when it was attached
    time: DateTime<Utc>,
    /// The gear the part is attached to
    gear: PartId,
    /// the hook on that gear
    hook: PartTypeId,
}

impl Event {
    fn attach(self, user: &dyn Person, conn: & AppConn) -> TbResult<Summary> {
        info!("attach {:?}", self);
        // check user
        let part = self.part_id.part(user, conn)?;
        // and types
        let mytype = part.what.get(conn)?;
        ensure!(
            mytype.hooks.contains(&self.hook),
            Error::BadRequest(format!("Type {} cannot be attached to hook {}", mytype.name, self.hook))
        );
        let gear = self.gear.part(user,conn)?;
        ensure!(
            mytype.main == gear.what,
            Error::BadRequest(format!("Type {} cannot be attached to gear {}", mytype.name, gear.what))
        );
        conn.transaction(|| {
            use schema::parts;
            use schema::attachments::dsl::*;
            let mut hash = SumHash::default();

            // make sure that no other part is attached at hook
            hash = hash.merge(self.detach_other(conn)?);
            
            let mut end = MAX_DATETIME;
            if let Some(next) = attachments
                .inner_join(
                    parts::table.on(parts::id
                        .eq(part_id) // join corresponding part
                        .and(parts::what.eq(part.what))),
                ) // where the part has my type
                .filter(gear.eq(self.gear))
                .filter(hook.eq(self.hook))
                .select(schema::attachments::all_columns) // return only the attachment
                .filter(attached.gt(self.time))
                .order(attached)
                .first::<Attachment>(conn).optional()? 
            {
                //something is already attached to the hook
                if next.part_id == self.part_id {
                    debug!("pred found {:?}", next);
                    // the next attachment gets an earlier start
                    // but the end stays the same!
                    end = next.detached;
                    hash = hash.merge(next.delete(conn)?);
                } else {
                    debug!("adding to the front of {:?}", next);
                    // the new attachment ends when the next starts
                    end = next.attached;
                }
            } else if let Some(next) = attachments.for_update()
                .filter(part_id.eq(self.part_id))
                .filter(attached.gt(self.time))
                .order(attached)
                .first::<Attachment>(conn).optional()? 
            {
                debug!("successor found {:?}", next);
                // the current part is attached somewhere else
                // in the future
                // so the attachment ends when the next starts
                end = next.attached;
            }

            if let Some(prev) = attachments.for_update()
                .filter(part_id.eq(self.part_id))
                .filter(attached.le(self.time)).filter(detached.gt(self.time))
                .first::<Attachment>(conn).optional()?
            {
                debug!("prolonging {:?}", prev);
                // the current part is already attached
                if prev.gear == self.gear && prev.hook == self.hook {
                    // the same part is already attached at the target hook
                    // so we need to adjust the end time
                    // and do not need to create a new attachment
                    let sum = prev.set_detach(end, conn)?;
                    return Ok(hash.merge(sum).collect());
                } else {
                    hash = hash.merge(prev.set_detach(self.time, conn)?);
                }
            }
            let sum = self.attachment(end).create(conn)?;
            Ok(hash.merge(sum).collect())
        })
    }

    /// create an attachment our of self with the given detached time
    fn attachment (self, detached: DateTime<Utc>) -> Attachment {
        Attachment {
            part_id: self.part_id,
            gear: self.gear,
            hook: self.hook,
            attached: self.time,
            detached,
            count: 0,
            time: 0,
            climb: 0,
            descend: 0,
            distance: 0
        }
    }

    fn detach(self, user: &dyn Person, conn: & AppConn) -> TbResult<Summary> {
        info!("detach {:?}", self);
        // check user
        self.part_id.checkuser(user, conn)?;
        let att = Attachment::get(self.part_id, self.time, conn)?;
        
        ensure!(
            self.part_id == att.part_id && self.hook == att.hook && self.gear == att.gear,
            Error::BadRequest(format!("{:?} does not match attachment", self))
        );
        conn.transaction(|| {
            att.set_detach(self.time, conn)
        })  
    }

    /// iff there is another part attached at Event, detach it
    fn detach_other(&self, conn: & AppConn) -> TbResult<Summary> {
        use schema::parts;
        use schema::attachments::dsl::*;
        let what= self.part_id.what(conn)?;
        match attachments
            .inner_join(
                parts::table.on(parts::id
                    .eq(part_id) // join corresponding part
                    .and(parts::what.eq(what))),
            ) // where the part has my type
            .filter(gear.eq(self.gear))
            .filter(hook.eq(self.hook))
            .select(schema::attachments::all_columns) // return only the attachment
            .filter(attached.le(self.time)).filter(detached.gt(self.time))
            .first::<Attachment>(conn).optional()? 
        {
            Some(prev) => prev.set_detach(self.time, conn),
            None => Ok(Summary::default())
        } 
    }
}
/// Timeline of attachments
///
/// Every attachment of a part to a specified hook on a gear is an entry
/// Start and end time are noted
///
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Queryable,
    Identifiable,
    Associations,
    Insertable,
    AsChangeset,
)]
#[primary_key(part_id, attached)]
#[changeset_options(treat_none_as_null = "true")]
#[belongs_to(PartType, foreign_key = "hook")]
// #[belongs_to(Part, foreign_key = "hook_id")]
pub struct Attachment {
    /// the sub-part, which is attached to the hook
    part_id: PartId,
    /// when it was attached
    attached: DateTime<Utc>,
    /// The gear the part is attached to
    gear: PartId,
    /// the hook on that gear
    hook: PartTypeId,
    /// when it was removed again, "none" means "still attached"
    detached: DateTime<Utc>,
    // we do not accept theses values from the client!
    /// usage count
    pub count: i32,
    /// usage time
    pub time: i32,
    /// Usage distance
    pub distance: i32,
    /// Overall climbing
    pub climb: i32,
    /// Overall descending
    pub descend: i32,
}

#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct AttachmentDetail {
    #[serde(flatten)]
    a: Attachment,
    name: String,
    what: PartTypeId,
}

impl AttachmentDetail {
    pub fn idx(&self) -> String {
        format!("{}{}", self.a.part_id, self.a.attached)
    }
}

impl Attachment {
    fn get(part: PartId, etime: DateTime<Utc>, conn: &AppConn) -> TbResult<Self> {
        use schema::attachments::dsl::*;

        Ok(attachments
            .order(attached.desc()) // Ordered by time
            .filter(part_id.eq(part)) // is the right part
            .filter(attached.le(etime))
            .filter(detached.gt(etime))
            .for_update() // cannot be boxed!
            .first::<Attachment>(conn)?)
    }

    /// remove the corresponding usage from part and reset attachment
    fn remove(&mut self, conn: &AppConn) -> TbResult<Part> {
        trace!("remove attachment {:?}", self);
        let usage = self.usage(Factor::Sub, conn);
        self.count = 0;
        self.time = 0;
        self.distance = 0;
        self.climb = 0;
        self.descend = 0;
        self.part_id.apply_usage(&usage, conn)
    }

    /// add the corresponding usage to part and set it in attachment
    fn add(&mut self, conn: &AppConn) -> TbResult<Part> {
        trace!("add attachment {:?}", self);
        let usage = self.usage(Factor::Add, conn);
        self.count = usage.count;
        self.time = usage.time;
        self.distance = usage.distance;
        self.climb = usage.climb;
        self.descend = usage.descend;
        self.part_id.apply_usage(&usage, conn)
    }

    /// return the usage for the attachment
    fn usage(&self, factor: Factor, conn: &AppConn) -> Usage {
        Activity::find(self.gear, self.attached, self.detached, conn)
            .into_iter()
            .fold(Usage::none(self.attached), |acc, x| {
                acc.add_activity(&x, factor)
            })
    }

    /// change detached time for attachment
    ///
    /// deletes the attachment for detached < attached
    /// Does not check for collisions
    fn set_detach(mut self, detached: DateTime<Utc>, conn: &AppConn) -> TbResult<Summary> {
        debug!("detaching {:?} at {:?}", self, detached);
        
        let del = self.delete(conn)?;
        if self.attached >= detached {
            return Ok(del);
        }
        
        self.detached = detached;
        let cre = self.create(conn)?;
        Ok(del.merge(cre))
    }

    /// register and store a new attachment
    fn create(mut self, conn: &AppConn) -> TbResult<Summary>
    {
        debug!("Create {:?}", self);
        let part = self.add(conn)?; // and register changes
        let a = self.insert_into(attachments::table)
            .get_result(conn).context("insert into attachments")?;
        let attachment = AttachmentDetail {
                a,
                name: part.name.clone(), 
                what: part.what
            };
        Ok(Summary {
            parts: vec![part], 
            attachments: vec![attachment],
            ..Default::default()
        })
    }

    /// deletes an attachment with its side-effects
    ///
    /// - recalculates the usage counters in the attached assembly
    /// - returns all affected parts
    fn delete(self, conn: &AppConn) -> TbResult<Summary> {
        debug!("Delete {:?}", self);
        let ctx = format!("Could not delete attachment {:#?}", self);
        let mut att = diesel::delete(attachments::table.find(self.id())) // delete the attachment in the database
            .get_result::<Attachment>(conn)
            .context(ctx)?;
        
        let part = att.remove(conn)?;
        // mark as deleted for client!
        att.detached = att.attached;
        return Ok(Summary{
            attachments: vec![att.add_details("".into(), 0.into())],
            parts: vec![part],
            activities: vec![]
        })
        
    }
   
    /// add redundant details for client simplicity
    fn add_details(self, name: String, what: PartTypeId) -> AttachmentDetail {
        AttachmentDetail {
            name,
            what,
            a: self
        }
    }

    /// read and add redundant details for client simplicity
    fn enrich(self, conn: &AppConn) -> TbResult<AttachmentDetail> {
        use schema::parts::dsl::{parts,name,what};
        
        let (n, w) = parts
            .find(self.part_id)
            .select((name,what))
            .get_result::<(String,PartTypeId)>(conn)?;
        Ok(self.add_details(n, w))
    }

    pub fn parts_per_activity(act: &Activity, conn: &AppConn) -> Vec<PartId> {
        use schema::attachments::dsl::*;
    
        let mut res = Vec::new();
        if let Some(act_gear) = act.gear {
            res.push(act_gear); // We need the gear too!
            res.append(
                &mut attachments
                    .filter(gear.eq(act_gear))
                    .filter(attached.lt(act.start))
                    .filter(detached.is_null().or(detached.ge(act.start)))
                    .select(part_id)
                    .get_results::<PartId>(conn)
                    .expect("Error reading attachments"),
            );
        }
        res
    }   
    
    pub fn register(act: &Activity, usage: &Usage, conn: &AppConn) -> Vec<AttachmentDetail> {
        use schema::attachments::dsl::*;
    
        if let Some(act_gear) = act.gear {
            diesel::update(
                attachments
                    .filter(gear.eq(act_gear))
                    .filter(attached.lt(act.start))
                    .filter(detached.ge(act.start))
            )
            .set((
                time.eq(time + usage.time),
                climb.eq(climb + usage.climb),
                descend.eq(descend + usage.descend),
                distance.eq(distance + usage.distance),
                count.eq(count + usage.count),
            ))
            .get_results::<Attachment>(conn).expect("Database Error")
            .into_iter()
            .map(|a| a.enrich(conn).expect("couldn't enrich attachment"))
            .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    }    
    
    pub fn for_parts(partlist: &Vec<Part>, conn: &AppConn) -> TbResult<Vec<AttachmentDetail>> {
        use schema::attachments::dsl::*;
        use schema::parts::dsl::{parts,id,name,what};
        let ids: Vec<_> = partlist.iter().map(|p| p.id).collect();
        let atts = attachments
        .filter(part_id.eq_any(ids.clone()))
        .or_filter(gear.eq_any(ids))
        .inner_join(parts.on(id.eq(part_id)))
        .select((schema::attachments::all_columns,name,what))
        .get_results::<AttachmentDetail>(conn)?;
        
        Ok(atts)
    }
}


#[post("/attach", data = "<event>")]
fn attach_rt(event: Json<Event>, user: &User, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(event.into_inner().attach(user, &conn))
}

#[post("/detach", data = "<event>")]
fn detach_rt(event: Json<Event>, user: &User, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(event.into_inner().detach(user, &conn))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![attach_rt, detach_rt]
}
