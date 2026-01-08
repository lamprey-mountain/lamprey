select count(*) from message where channel_id = $1 and deleted_at is not null
