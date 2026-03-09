alter table role add column version_id uuid;
update role set version_id = id;
alter table role alter column version_id set not null;
