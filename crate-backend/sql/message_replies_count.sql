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
        depth,
        row_number() over (partition by reply_id order by id) as rn
    from
        message_tree
),
filtered_messages as (
    select id
    from ranked_messages
    where (depth = 1 or rn <= $4 or $4 is null)
)
select count(*) from filtered_messages where id in (select id from message where channel_id = $1 and deleted_at is null and is_latest)
