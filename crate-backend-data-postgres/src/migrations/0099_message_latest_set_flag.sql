update message set is_latest = true where version_id in (select max(version_id) from message group by id);
