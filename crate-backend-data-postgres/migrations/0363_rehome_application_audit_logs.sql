-- `target_application_id` is the application an audit log entry is about
alter table audit_log add column target_application_id uuid;

update audit_log
set target_application_id = (data->'metadata'->>'application_id')::uuid
where data->>'type' in ('ApplicationCreate', 'ApplicationUpdate', 'ApplicationDelete')
  and data->'metadata'->>'application_id' is not null;

create index audit_log_room_id_target_application_id_idx
on audit_log (room_id, target_application_id);

-- session delete all entries did not use to have a metadata object
update audit_log
set data = jsonb_set(data, '{metadata}', '{}'::jsonb, true)
where data->>'type' = 'SessionDeleteAll'
  and not data ? 'metadata';

-- audit log entries are stored for both the application and its owner.
-- copy existing application entries to the owner
with candidates as (
    select
        al.*,
        app.owner_id,
        md5(al.id::text || clock_timestamp()::text || random()::text) as r
    from audit_log al
    join application app on app.id = al.room_id
    where al.room_id <> app.owner_id
      and not exists (
          select 1
          from audit_log x
          where x.room_id = app.owner_id
            and x.user_id = al.user_id
            and x.data = al.data
            and x.started_at = al.started_at
      )
),
copies as (
    select
        candidates.*,
        (
            substring(id::text, 1, 14)
            || '7'
            || substring(r, 1, 3)
            || '-'
            || substring(r, 4, 4)
            || '-'
            || substring(r, 8, 12)
        )::uuid as new_id
    from candidates
)
insert into audit_log (
    id,
    room_id,
    user_id,
    session_id,
    reason,
    data,
    status,
    started_at,
    ended_at,
    ip_addr,
    user_agent,
    application_id,
    target_application_id,
    channel_id,
    target_user_id
)
select
    new_id,
    owner_id,
    user_id,
    session_id,
    reason,
    data,
    status,
    started_at,
    ended_at,
    ip_addr,
    user_agent,
    application_id,
    target_application_id,
    channel_id,
    target_user_id
from copies;

-- copy existing application events/owner logins to the application
with app_events as (
    select
        al.*,
        (al.data->'metadata'->>'application_id')::uuid as app_id
    from audit_log al
    where al.data->>'type' in ('ApplicationCreate', 'ApplicationUpdate', 'ApplicationDelete')
      and al.data->'metadata'->>'application_id' is not null

    union all

    select
        al.*,
        al.target_user_id as app_id
    from audit_log al
    where al.data->>'type' = 'SessionLogin'
      and al.target_user_id in (select id from application)
),
candidates as (
    select
        app_events.*,
        md5(app_events.id::text || clock_timestamp()::text || random()::text) as r
    from app_events
    join application app on app.id = app_events.app_id
    where app_events.room_id = app.owner_id
      and app_events.room_id <> app_events.app_id
      and not exists (
          select 1
          from audit_log x
          where x.room_id = app_events.app_id
            and x.user_id = app_events.user_id
            and x.data = app_events.data
            and x.started_at = app_events.started_at
      )
),
copies as (
    select
        candidates.*,
        (
            substring(id::text, 1, 14)
            || '7'
            || substring(r, 1, 3)
            || '-'
            || substring(r, 4, 4)
            || '-'
            || substring(r, 8, 12)
        )::uuid as new_id
    from candidates
)
insert into audit_log (
    id,
    room_id,
    user_id,
    session_id,
    reason,
    data,
    status,
    started_at,
    ended_at,
    ip_addr,
    user_agent,
    application_id,
    target_application_id,
    channel_id,
    target_user_id
)
select
    new_id,
    app_id,
    user_id,
    session_id,
    reason,
    data,
    status,
    started_at,
    ended_at,
    ip_addr,
    user_agent,
    application_id,
    target_application_id,
    channel_id,
    target_user_id
from copies;
