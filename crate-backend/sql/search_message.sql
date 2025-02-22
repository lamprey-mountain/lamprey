with
    message_coalesced as (
        select *
        from (select *, row_number() over(partition by id order by version_id desc) as row_num
            from message)
        where row_num = 1
    ),
    thread_viewer as (
        select thread.id from thread
        join room_member on thread.room_id = room_member.room_id
        where room_member.user_id = $1
    ),
    embeds as (
        select version_id, array_agg(row_to_json(u)) as embeds
        from url_embed_message u
        group by version_id
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
    row_to_json(usr) as "author!: serde_json::Value",
    coalesce(att_json.attachments, '{}') as "attachments!",
    false as "is_pinned!",
    coalesce(embeds.embeds, '{}') as "embeds!"
from message_coalesced as msg
join usr on usr.id = msg.author_id
join thread_viewer on msg.thread_id = thread_viewer.id
left join att_json on att_json.version_id = msg.version_id
left join embeds on embeds.version_id = msg.version_id
where msg.deleted_at is null
  and msg.id > $2 AND msg.id < $3
  and content @@ websearch_to_tsquery($6)
order by (CASE WHEN $4 = 'f' THEN msg.id END), msg.id DESC LIMIT $5
