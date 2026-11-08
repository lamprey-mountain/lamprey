alter table audit_log add column target_user_id uuid;

-- backfill from the audit log entry metadata
update audit_log
set target_user_id = (data->'metadata'->>'user_id')::uuid
where target_user_id is null
    and data->'metadata'->>'user_id' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$';

update audit_log
set target_user_id = (data->'metadata'->>'bot_id')::uuid
where target_user_id is null
    and data->'metadata'->>'bot_id' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$';

-- permission overwrites only target a user when the overwrite is for a user
update audit_log
set target_user_id = (data->'metadata'->>'overwrite_id')::uuid
where target_user_id is null
    and data->>'type' like 'PermissionOverwrite%'
    and data->'metadata'->>'type' = 'User'
    and data->'metadata'->>'overwrite_id' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$';

create index audit_log_room_id_target_user_id_idx on audit_log (room_id, target_user_id);
