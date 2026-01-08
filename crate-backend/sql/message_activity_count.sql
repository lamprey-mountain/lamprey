select count(*) from message m
join message_version mv on m.latest_version_id = mv.version_id
where m.channel_id = $1 and m.deleted_at is null and mv.type IN ('MessagePinned', 'MemberAdd', 'MemberRemove', 'ThreadRename', 'ChannelIcon')
