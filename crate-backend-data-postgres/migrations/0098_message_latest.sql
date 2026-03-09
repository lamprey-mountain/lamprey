alter table message add column is_latest boolean not null default false;
alter table message alter column is_latest drop default;
create unique index message_is_latest on message (id) where is_latest;
