-- the latest sync sequence number in this channel
alter table channel add column latest_seq bigint not null default 0;

-- bumped on create, delete, remove, restore
alter table message add column seq bigint not null default 0;

-- bumped on create (aka edits) and delete
alter table message_version add column seq bigint not null default 0;

-- used for recreating reaction events
alter table reaction add column created_seq bigint not null default 0;
alter table reaction add column deleted_seq bigint;
