alter table message rename to message_old;

create table message (
    id uuid primary key,
    channel_id uuid not null references channel(id),
    author_id uuid not null references usr(id),
    created_at timestamp not null,
    pinned jsonb,
    deleted_at timestamp,
    removed_at timestamp,
    -- temporarily hide constraint to prevent recursive dependency issue
    latest_version_id uuid not null
);

create table message_version (
    version_id uuid primary key,
    message_id uuid not null references message(id),
    author_id uuid not null references usr(id),
    type message_type not null,
    content text,
    metadata jsonb,
    reply_id uuid,
    mentions jsonb,
    embeds jsonb,
    created_at timestamp not null,
    deleted_at timestamp,

    -- this isn't use anymore, but i'd rather not lose any data if i can help it
    override_name text
);

-- migrate data
insert into message (id, channel_id, author_id, created_at, pinned, deleted_at, removed_at, latest_version_id)
select distinct on (id) id, channel_id, author_id, coalesce(created_at, extract_timestamp_from_uuid_v7(id)) as created_at, pinned, deleted_at, removed_at, version_id
from message_old
where is_latest;

insert into message_version (version_id, message_id, author_id, type, content, metadata, reply_id, mentions, embeds, created_at, deleted_at, override_name)
select version_id, id, author_id, type, content, metadata, reply_id, mentions, embeds, coalesce(created_at, extract_timestamp_from_uuid_v7(version_id)) as edited_at, deleted_at, override_name from message_old;

-- readd foreign key constraints
alter table message add constraint fk_message_latest_version_id foreign key (latest_version_id) references message_version(version_id);
alter table message_attachment add constraint fk_message_version foreign key (version_id) references message_version(version_id);

-- this has extremely poor performance anyways...
drop view hydrated_mentions;

-- drop the last pointers to the old table
alter table message_attachment drop constraint message_attachment_version_id_fkey;

-- goodbye, old schema!
drop table message_old;
