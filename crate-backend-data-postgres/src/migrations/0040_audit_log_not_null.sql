alter table audit_log alter column id set not null;
alter table audit_log alter column room_id set not null;
alter table audit_log alter column user_id set not null;
alter table audit_log alter column reason set not null;
alter table audit_log alter column payload set not null;
