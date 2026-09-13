use super::*;
use crate::{Error, Part, PartId, PartStore, PartTypeId, ShopId, TbResult, UsageId, UserId};
use async_trait::async_trait;
use time::OffsetDateTime;

#[async_trait]
impl PartStore for MemStore {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        self.parts
            .get(&pid)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Part {} not found", pid)))
    }

    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        Ok(self
            .parts
            .values()
            .filter(|p| &p.owner == uid)
            .cloned()
            .collect())
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
        let id = PartId::from(self.next_part_id);
        self.next_part_id += 1;
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
        self.parts.insert(id, part.clone());
        Ok(part)
    }

    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        match self.parts.insert(part.id, part.clone()) {
            Some(_) => Ok(part),
            None => Err(Error::NotFound(format!("Part {} not found", part.id))),
        }
    }

    async fn part_delete(&mut self, part: PartId) -> TbResult<PartId> {
        match self.parts.remove(&part) {
            Some(_) => Ok(part),
            None => Err(Error::NotFound(format!("Part {} not found", part))),
        }
    }

    async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize> {
        let mut count = 0;
        for part in parts {
            if self.parts.remove(&part.id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        Ok(self
            .parts
            .values()
            .find(|p| p.source.as_deref() == Some(strava_id))
            .map(|p| p.id))
    }

    async fn parts_register_shop(
        &mut self,
        shop_id: ShopId,
        part_ids: Vec<PartId>,
    ) -> TbResult<Vec<Part>> {
        let mut result = Vec::new();
        for pid in part_ids {
            if let Some(part) = self.parts.get_mut(&pid) {
                part.shop = Some(shop_id);
                result.push(part.clone());
            }
        }
        Ok(result)
    }

    async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>> {
        let mut result = Vec::new();
        for pid in part_ids {
            if let Some(part) = self.parts.get_mut(&pid) {
                part.shop = None;
                result.push(part.clone());
            }
        }
        Ok(result)
    }

    async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>> {
        Ok(self
            .parts
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
        let id = PartNoteId::from(self.next_note_id);
        self.next_note_id += 1;
        let note = PartNote {
            id,
            part,
            kind: NoteKind::Text,
            name,
            mime: None,
            filename: None,
            size: None,
            created,
        };
        self.part_notes.insert(id, note.clone());
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
        let id = PartNoteId::from(self.next_note_id);
        self.next_note_id += 1;
        let note = PartNote {
            id,
            part,
            kind: NoteKind::File,
            name,
            mime: Some(mime),
            filename,
            size: Some(size),
            created,
        };
        self.note_files.insert(id, data);
        self.part_notes.insert(id, note.clone());
        Ok(note)
    }

    async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>> {
        Ok(self
            .part_notes
            .values()
            .filter(|n| n.part == part)
            .cloned()
            .collect())
    }

    async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        self.part_notes
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("PartNote {id} not found")))
    }

    async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>> {
        self.note_files
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("PartNote file {id} not found")))
    }

    async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote> {
        let note = self
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
        let note = self
            .part_notes
            .get_mut(&id)
            .ok_or_else(|| Error::NotFound(format!("PartNote {id} not found")))?;
        note.kind = NoteKind::File;
        note.name = name;
        note.mime = Some(mime);
        note.filename = filename;
        note.size = Some(size);
        self.note_files.insert(id, data);
        Ok(note.clone())
    }

    async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        let note = self
            .part_notes
            .get_mut(&id)
            .ok_or_else(|| Error::NotFound(format!("PartNote {id} not found")))?;
        note.kind = NoteKind::Text;
        note.mime = None;
        note.filename = None;
        note.size = None;
        self.note_files.remove(&id);
        Ok(note.clone())
    }

    async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId> {
        match self.part_notes.remove(&id) {
            Some(_) => {
                self.note_files.remove(&id);
                Ok(id)
            }
            None => Err(Error::NotFound(format!("PartNote {id} not found"))),
        }
    }
}
