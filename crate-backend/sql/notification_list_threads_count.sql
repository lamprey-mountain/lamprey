with unread_channels as (
    select distinct channel_id from inbox
    where user_id = $1
    and ($2 or read_at is null)
    and (array_length($3::uuid[], 1) is null or room_id = any($3))
    and (array_length($4::uuid[], 1) is null or channel_id = any($4))
)
select count(*) from unread_channels
