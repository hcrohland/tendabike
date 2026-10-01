use super::*;
use crate::{Error, Part, PartId, PartStore, PartTypeId, ShopId, TbResult, UsageId, UserId};
use async_trait::async_trait;
use time::OffsetDateTime;

#[async_trait]
impl PartStore for MemStore {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        let d = self.state();
        d.parts
            .get(&pid)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Part {} not found", pid)))
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        let d = self.state();
        // One rule on both stores (issue #405): the database lists a user's
        // parts `ORDER BY last_used`, so this in-memory mirror applies the
        // same sort; ties have no defined order.
        let mut parts: Vec<Part> = d
            .parts
            .values()
            .filter(|p| &p.owner == uid)
            .cloned()
            .collect();
        parts.sort_by_key(|p| p.last_used);
        Ok(parts)
    }

    async fn part_create(
        &mut self,
        what: PartTypeId,
        name: String,
        vendor: String,
        model: String,
        purchase: OffsetDateTime,
        source: Option<String>,
        usage: UsageId,
        owner: UserId,
        shop: Option<ShopId>,
    ) -> TbResult<Part> {
        let d = self.state_mut();
        let id = PartId::from(d.next_part_id);
        d.next_part_id += 1;
        let part = Part {
            id,
            owner,
            what,
            name,
            vendor,
            model,
            purchase,
            last_used: purchase,
            disposed_at: None,
            usage,
            source,
            shop,
        };
        d.parts.insert(id, part.clone());
        Ok(part)
    }

    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        let d = self.state_mut();
        match d.parts.insert(part.id, part.clone()) {
            Some(_) => Ok(part),
            None => Err(Error::NotFound(format!("Part {} not found", part.id))),
        }
    }

    async fn part_delete(&mut self, part: PartId) -> TbResult<PartId> {
        let d = self.state_mut();
        match d.parts.remove(&part) {
            Some(_) => Ok(part),
            None => Err(Error::NotFound(format!("Part {} not found", part))),
        }
    }

    async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize> {
        let d = self.state_mut();
        let mut count = 0;
        for part in parts {
            if d.parts.remove(&part.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        let d = self.state();
        Ok(d.parts
            .values()
            .find(|p| p.source.as_deref() == Some(strava_id))
            .map(|p| p.id))
    }

    async fn parts_register_shop(
        &mut self,
        shop_id: ShopId,
        part_ids: Vec<PartId>,
    ) -> TbResult<Vec<Part>> {
        let d = self.state_mut();
        let mut result = Vec::new();
        for pid in part_ids {
            if let Some(part) = d.parts.get_mut(&pid) {
                part.shop = Some(shop_id);
                result.push(part.clone());
            }
        }
        Ok(result)
    }

    async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>> {
        let d = self.state_mut();
        let mut result = Vec::new();
        for pid in part_ids {
            if let Some(part) = d.parts.get_mut(&pid) {
                part.shop = None;
                result.push(part.clone());
            }
        }
        Ok(result)
    }

    async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>> {
        let d = self.state();
        Ok(d.parts
            .values()
            .filter(|p| p.shop.as_ref() == Some(&shop_id))
            .cloned()
            .collect())
    }
}

#[async_trait]
impl PartNoteStore for MemStore {
    async fn partnote_create_text(
        &mut self,
        part: PartId,
        name: String,
        created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        let d = self.state_mut();
        let id = PartNoteId::from(d.next_note_id);
        d.next_note_id += 1;
        let note = PartNote {
            id,
            part,
            name,
            mime: None,
            filename: None,
            size: None,
            created,
        };
        d.part_notes.insert(id, note.clone());
        Ok(note)
    }

    async fn partnote_create_file(
        &mut self,
        part: PartId,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
        created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        let d = self.state_mut();
        let id = PartNoteId::from(d.next_note_id);
        d.next_note_id += 1;
        let note = PartNote {
            id,
            part,
            name,
            mime: Some(mime),
            filename,
            size: Some(size),
            created,
        };
        d.note_files.insert(id, data);
        d.part_notes.insert(id, note.clone());
        Ok(note)
    }

    async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>> {
        let d = self.state();
        Ok(d.part_notes
            .values()
            .filter(|n| n.part == part)
            .cloned()
            .collect())
    }

    async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        let d = self.state();
        d.part_notes
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("PartNote {id} not found")))
    }

    async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>> {
        let d = self.state();
        d.note_files
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("PartNote file {id} not found")))
    }

    async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote> {
        let d = self.state_mut();
        let note = d
            .part_notes
            .get_mut(&id)
            .ok_or_else(|| Error::NotFound(format!("PartNote {id} not found")))?;
        note.name = name;
        Ok(note.clone())
    }

    async fn partnote_update_file(
        &mut self,
        id: PartNoteId,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
    ) -> TbResult<PartNote> {
        let d = self.state_mut();
        let note = d
            .part_notes
            .get_mut(&id)
            .ok_or_else(|| Error::NotFound(format!("PartNote {id} not found")))?;
        note.name = name;
        note.mime = Some(mime);
        note.filename = filename;
        note.size = Some(size);
        d.note_files.insert(id, data);
        Ok(note.clone())
    }

    async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        let d = self.state_mut();
        let note = d
            .part_notes
            .get_mut(&id)
            .ok_or_else(|| Error::NotFound(format!("PartNote {id} not found")))?;
        note.mime = None;
        note.filename = None;
        note.size = None;
        d.note_files.remove(&id);
        Ok(note.clone())
    }

    async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId> {
        let d = self.state_mut();
        match d.part_notes.remove(&id) {
            Some(_) => {
                d.note_files.remove(&id);
                Ok(id)
            }
            None => Err(Error::NotFound(format!("PartNote {id} not found"))),
        }
    }
}
