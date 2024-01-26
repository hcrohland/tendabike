-- Your SQL goes here

ALTER TABLE "public"."activities" DROP CONSTRAINT "activities_what_fkey";
ALTER TABLE "public"."attachments" DROP CONSTRAINT "attachments2_hook_fkey";
ALTER TABLE "public"."parts" DROP CONSTRAINT "parts_what_fkey";

drop table activity_types;
drop table part_types;