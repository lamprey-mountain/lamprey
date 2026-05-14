create type room_feature as enum ('Scripts');
alter table room add column features room_feature[] not null default '{}';
