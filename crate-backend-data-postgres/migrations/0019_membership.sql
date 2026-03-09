create type membership as enum ('Join', 'Ban');
drop view member_json;
alter table room_member drop column membership;
alter table room_member add column membership membership not null default 'Join';
alter table room_member alter column membership drop default;
