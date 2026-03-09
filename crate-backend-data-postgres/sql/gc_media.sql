update media set deleted_at = now()
where id not in (select media_id from media_link)
and extract_timestamp_from_uuid_v7(id) < now() - interval '7 day';
