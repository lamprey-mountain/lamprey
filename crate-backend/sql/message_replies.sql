with recursive message_tree as (
    select
        id,
        reply_id,
        1 as depth
    from
        message
    where
        ($2::uuid is not null and id = $2::uuid)
        or ($2::uuid is null and reply_id is null)
    union all
    select
        m.id,
        m.reply_id,
        mt.depth + 1
    from
        message m
        join message_tree mt on m.reply_id = mt.id
    where
        mt.depth < $3
),
ranked_messages as (
    select
        id,
        reply_id,
        row_number() over (partition by reply_id order by id) as rn
    from
        message_tree
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
    hm.mentions,
    coalesce(att_json.attachments, '{}') as "attachments!",
    msg.embeds as "embeds"
from message as msg
join ranked_messages rm on msg.id = rm.id
left join att_json on att_json.version_id = msg.version_id
left join hydrated_mentions hm on hm.message_id = msg.id
where is_latest and channel_id = $1 and msg.deleted_at is null and (rm.rn <= $4 or $4 is null)
  and msg.id > $5 AND msg.id < $6
order by (CASE WHEN $7 = 'f' THEN msg.id END), msg.id DESC LIMIT $8
