create index media_remote_lookup on media (remote_hostname, remote_origin_id) where remote_origin_id is not null;
