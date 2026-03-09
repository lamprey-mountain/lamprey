alter table message add column created_at timestamp;
update message set created_at = extract_timestamp_from_uuid_v7(version_id);
alter table message alter column created_at set not null;
