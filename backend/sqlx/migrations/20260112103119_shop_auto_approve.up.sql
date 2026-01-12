-- Add up migration script here
alter table shops add column if not exists auto_approve boolean not null default false ;