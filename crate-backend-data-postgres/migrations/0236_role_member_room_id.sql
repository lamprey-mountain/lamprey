alter table role_member add column room_id uuid;
update role_member rm set room_id = r.room_id from role r where r.id = rm.role_id;
alter table role_member alter column room_id set not null;
create index on role_member (room_id);
