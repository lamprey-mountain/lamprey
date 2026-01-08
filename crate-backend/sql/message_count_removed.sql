select count(*) from message where channel_id = $1 and removed_at is not null
