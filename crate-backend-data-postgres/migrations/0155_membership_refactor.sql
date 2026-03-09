alter table room_member add column joined_at timestamp;
update room_member set joined_at = membership_updated_at where membership = 'Join';
delete from room_member where joined_at is null;
alter table room_member alter column joined_at set not null;

alter table thread_member add column joined_at timestamp;
update thread_member set joined_at = membership_updated_at where membership = 'Join';
delete from thread_member where joined_at is null;
alter table thread_member alter column joined_at set not null;
