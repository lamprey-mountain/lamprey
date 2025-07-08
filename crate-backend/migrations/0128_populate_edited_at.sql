update message set edited_at = extract_timestamp_from_uuid_v7(version_id) where version_id != id;
