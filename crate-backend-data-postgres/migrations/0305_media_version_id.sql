alter table media add column version_id uuid;
update media set version_id = id;
alter table media alter column version_id set not null;
create index media_version_id_idx on media (version_id);
