-- rename seq to created_seq for clarity
alter table message rename column seq to created_seq;
alter table message_version rename column seq to created_seq;

-- tracks lifecycle events (delete, remove, restore)
-- for initial creation, this should match created_seq
alter table message add column lifecycle_seq bigint not null default 0;

-- update existing lifecycle_seq to match created_seq
update message set lifecycle_seq = created_seq;
