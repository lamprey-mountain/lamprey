select count(*) from message where channel_id = $1 and pinned is not null and deleted_at is null and is_latest;
