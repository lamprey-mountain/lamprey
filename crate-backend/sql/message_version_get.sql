with
reaction_counts as (
    select message_id, key, min(position) as pos, count(*) as count, bool_or(user_id = $2) as self_reacted
    from reaction
    group by message_id, key
),
message_reaction as (
    select message_id,
        json_agg(jsonb_build_object(
            'key', key,
            'count', count,
            'self', self_reacted
        ) order by pos) as json
    from reaction_counts
    group by message_id
),
hydrated_mentions as (
    select
        (SELECT jsonb_agg(jsonb_build_object('id', u.id, 'resolved_name', COALESCE(rm.override_name, u.name))) FROM jsonb_array_elements_text(msg.mentions->'users') AS uid JOIN usr u ON u.id = uid::uuid LEFT JOIN room_member rm ON rm.user_id = u.id AND rm.room_id = ch.room_id) as users,
        (SELECT jsonb_agg(jsonb_build_object('id', r.id)) FROM jsonb_array_elements_text(msg.mentions->'roles') AS rid JOIN role r ON r.id = rid::uuid) as roles,
        (SELECT jsonb_agg(jsonb_build_object('id', c.id, 'room_id', c.room_id, 'type', c.type, 'name', c.name)) FROM jsonb_array_elements_text(msg.mentions->'channels') AS cid JOIN channel c ON c.id = cid::uuid) as channels,
        (SELECT jsonb_agg(jsonb_build_object('id', e.id, 'name', e.name, 'animated', e.animated)) FROM jsonb_array_elements_text(msg.mentions->'emojis') AS eid JOIN custom_emoji e ON e.id = eid::uuid) as emojis
    from message msg
    join channel ch on msg.channel_id = ch.id
    where msg.channel_id = $1 and msg.version_id = $3 and msg.deleted_at is null
)
select
    msg.type as "message_type: DbMessageType",
    msg.id,
    msg.channel_id,
    msg.version_id,
    msg.ordering,
    msg.content,
    msg.metadata,
    msg.reply_id,
    msg.override_name,
    msg.author_id,
    msg.created_at,
    msg.edited_at,
    msg.deleted_at,
    msg.removed_at,
    msg.pinned,
    jsonb_build_object(
        'users', COALESCE((select users from hydrated_mentions), '[]'::jsonb),
        'roles', COALESCE((select roles from hydrated_mentions), '[]'::jsonb),
        'channels', COALESCE((select channels from hydrated_mentions), '[]'::jsonb),
        'emojis', COALESCE((select emojis from hydrated_mentions), '[]'::jsonb),
        'everyone', msg.mentions->'everyone'
    ) as mentions,
    coalesce(att_json.attachments, '{}') as "attachments!",
    msg.embeds as "embeds",
    r.json as "reactions"
from message as msg
left join att_json on att_json.version_id = msg.version_id
left join message_reaction r on r.message_id = msg.id
where channel_id = $1 and msg.version_id = $3 and msg.deleted_at is null
