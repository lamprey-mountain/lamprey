with recursive message_tree as (
    select
        id,
        reply_id,
        1 as depth
    from
        message
    where
        id = $2
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
)
select count(*) from message_tree where id in (select id from message where thread_id = $1 and deleted_at is null and is_latest)
