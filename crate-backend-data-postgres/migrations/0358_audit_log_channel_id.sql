alter table audit_log add column channel_id uuid;

-- backfill from the audit log entry metadata
update audit_log
set channel_id = (data->'metadata'->>'channel_id')::uuid
where channel_id is null
    and data->'metadata'->>'channel_id' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$';

-- threads are channels too
update audit_log
set channel_id = (data->'metadata'->>'thread_id')::uuid
where channel_id is null
    and data->'metadata'->>'thread_id' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$';

create index audit_log_room_id_channel_id_idx on audit_log (room_id, channel_id);
