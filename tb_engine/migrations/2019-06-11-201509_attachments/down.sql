drop table attachments;

ALTER TABLE "parts"
  ADD COLUMN "attached_to" integer,
  ADD FOREIGN KEY ("attached_to") REFERENCES "parts"("id");
