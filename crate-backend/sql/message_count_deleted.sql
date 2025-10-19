select count(*) from message where channel_id = $1 and is_latest and deleted_at is not null
