-- Your SQL goes here
ALTER TABLE "parts"
  ADD COLUMN "last_used" timestamp with time zone NOT NULL DEFAULT 'epoch',
  ADD COLUMN "disposed_at" timestamp with time zone;

update parts set last_used = purchase;