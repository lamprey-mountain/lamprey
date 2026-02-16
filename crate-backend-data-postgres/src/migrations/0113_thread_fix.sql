alter table thread add column if not exists deleted_at timestamp;
alter table thread drop column if exists state;
alter table thread drop column if exists visibility;
