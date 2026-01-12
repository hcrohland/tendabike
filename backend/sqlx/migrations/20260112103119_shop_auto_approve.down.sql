-- Add down migration script here
alter table shops drop column if exists auto_approve;
