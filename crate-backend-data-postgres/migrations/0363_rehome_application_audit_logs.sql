-- application audit log entries are stored under the application's user id
update audit_log
set room_id = (data->'metadata'->>'application_id')::uuid
where data->>'type' in ('ApplicationCreate', 'ApplicationUpdate', 'ApplicationDelete')
  and data->'metadata'->>'application_id' is not null
  and room_id <> (data->'metadata'->>'application_id')::uuid;

-- session logins for application sessions belong to the application too
update audit_log
set room_id = target_user_id
where data->>'type' = 'SessionLogin'
  and target_user_id in (select id from application)
  and room_id <> target_user_id;
