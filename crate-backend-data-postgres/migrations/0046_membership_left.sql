alter type membership add value 'left';
alter table room_member rename column joined_at to membership_changed_at;
