alter table thread alter column position drop not null;
update thread set position = null;
