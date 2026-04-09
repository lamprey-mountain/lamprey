-- message sync: filter by channel + created_seq (new messages)
create index idx_message_channel_created_seq
    on message (channel_id, created_seq);

-- message sync: filter by channel + lifecycle_seq (delete/remove/restore events)
create index idx_message_channel_lifecycle_seq
    on message (channel_id, lifecycle_seq);

-- message version sync: filter by channel + created_seq (edits, version deletes)
create index idx_message_version_channel_created_seq
    on message_version (message_id, created_seq);

-- reaction sync: filter by channel + created_seq (reaction creates)
create index idx_reaction_channel_created_seq
    on reaction (channel_id, created_seq);

-- reaction sync: filter by channel + deleted_seq (reaction deletes)
create index idx_reaction_channel_deleted_seq
    on reaction (channel_id, deleted_seq)
    where deleted_seq is not null;

-- backfill reaction.channel_id from message.channel_id where null
-- (for reactions created before migration 0243)
update reaction
set channel_id = m.channel_id
from message m
where reaction.message_id = m.id
  and reaction.channel_id is null;

-- for some unknown reason, there were some reactions that pointed to missing messages in my test data
-- clean them up to prevent an error running this transaction
delete from reaction
where channel_id is null
  and not exists (
    select 1 from message where id = reaction.message_id
  );

-- now enforce not null
alter table reaction alter column channel_id set not null;
