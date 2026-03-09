ALTER TABLE metric_channel ADD COLUMN deleted_at TIMESTAMP;
ALTER TABLE metric_room ADD COLUMN deleted_at TIMESTAMP;
ALTER TABLE metric_invite ADD COLUMN deleted_at TIMESTAMP;
ALTER TABLE audit_log ADD COLUMN deleted_at TIMESTAMP;
