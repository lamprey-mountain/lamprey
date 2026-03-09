alter table message alter column deleted_at type timestamp using to_timestamp(deleted_at);
