update thread set version_id = coalesce(version_id, id);
update room set version_id = coalesce(version_id, id);
alter table thread alter column version_id set not null;
alter table room alter column version_id set not null;
