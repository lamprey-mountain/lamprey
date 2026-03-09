create type room_type as ENUM ('Default');
create type thread_type as ENUM ('Default');
create type message_type as ENUM ('Default', 'ThreadUpdate');
alter table room drop column type;
alter table thread drop column type;
alter table room add column type room_type not null default 'Default';
alter table thread add column type thread_type not null default 'Default';
drop view message_json_no_coalesce; -- forgot to drop this earlier
alter table message alter column type type message_type using case type when 0 then 'Default'::message_type when 1 then 'ThreadUpdate'::message_type end;
