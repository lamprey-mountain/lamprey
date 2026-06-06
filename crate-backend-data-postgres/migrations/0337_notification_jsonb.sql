alter table inbox add column info jsonb;

update inbox set info = jsonb_build_object(
    'type', type,
    'room_id', room_id,
    'channel_id', channel_id,
    'message_id', message_id,
    'target_user_id', target_user_id,
    'reaction_key', reaction_key
);

alter table inbox alter column info set not null;

alter table inbox drop column type;
alter table inbox drop column message_id;
alter table inbox drop column room_id;
alter table inbox drop column channel_id;
alter table inbox drop column reaction_key;
alter table inbox drop column target_user_id;

drop type notification_type;
