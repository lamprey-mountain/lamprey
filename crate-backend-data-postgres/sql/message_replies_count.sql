with recursive message_tree as (
    select
        m.id,
        mv.reply_id,
        1 as depth
    from
        message m
        join message_version mv on m.latest_version_id = mv.version_id
    where
        ($2::uuid is not null and m.id = $2::uuid)
        or ($2::uuid is null and mv.reply_id is null)
    union all
    select
        m.id,
        mv2.reply_id,
        mt.depth + 1
    from
        message m
        join message_version mv2 on m.latest_version_id = mv2.version_id
        join message_tree mt on mv2.reply_id = mt.id
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
select count(*) from filtered_messages where id in (select id from message where channel_id = $1 and deleted_at is null)
