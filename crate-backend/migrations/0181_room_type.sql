drop type room_type;
create type room_type as enum ('Default', 'Server');
alter table room add column type room_type not null default 'Default';
alter table room alter column type drop default;
