select count(*) from message where thread_id = $1 and is_latest
