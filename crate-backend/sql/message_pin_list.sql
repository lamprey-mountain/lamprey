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
)
select
    msg.type as "message_type: DbMessageType",
    msg.id,
    msg.thread_id,
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
    msg.mentions,
    coalesce(att_json.attachments, '{}') as "attachments!",
    msg.embeds as "embeds",
    r.json as "reactions"
from message as msg
left join att_json on att_json.version_id = msg.version_id
left join message_reaction r on r.message_id = msg.id
where is_latest and thread_id = $1 and msg.deleted_at is null and msg.pinned is not null
  and msg.id > $3 AND msg.id < $4
order by (CASE WHEN $5 = 'f' THEN (msg.pinned->>'position')::int END), (msg.pinned->>'position')::int DESC, (CASE WHEN $5 = 'f' THEN msg.id END), msg.id DESC
LIMIT $6
