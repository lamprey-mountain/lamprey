select count(*) from message where channel_id = $1 and is_latest and removed_at is not null
