create table script (
    id uuid primary key,
    channel_id uuid not null references channel(id) on delete cascade,
    creator_id uuid not null references usr(id),
    created_at timestamp not null,
    deleted_at timestamp,
    data jsonb not null
);

create table script_version (
    version_id uuid primary key,
    script_id uuid not null references script(id) on delete cascade,
    channel_id uuid not null references channel(id) on delete cascade,
    creator_id uuid not null references usr(id),
    created_at timestamp not null,
    deleted_at timestamp,
    data jsonb not null,
    cached_inputs jsonb
);

create index idx_script_channel on script(channel_id);
create index idx_script_not_deleted on script(channel_id) where deleted_at is not null;
create index idx_script_version_script on script_version(script_id);
create index idx_script_version_channel on script_version(channel_id);
