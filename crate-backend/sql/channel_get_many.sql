SELECT
    t.id, t.room_id, t.creator_id, t.owner_id, t.version_id, t.name, t.description, t.icon, t.type as "ty: _", t.nsfw, t.locked, t.archived_at, t.deleted_at, t.parent_id, t.position, t.bitrate, t.user_limit, t.invitable,
    (SELECT coalesce(COUNT(*), 0) FROM thread_member WHERE channel_id = t.id AND membership = 'Join') AS "member_count!",
    (SELECT version_id FROM message WHERE channel_id = t.id AND deleted_at IS NULL ORDER BY id DESC LIMIT 1) as last_version_id,
    (SELECT coalesce(COUNT(*), 0) FROM message WHERE channel_id = t.id AND deleted_at IS NULL) AS "message_count!",
    coalesce((SELECT json_agg(json_build_object('id', actor_id, 'type', type, 'allow', allow, 'deny', deny)) FROM permission_overwrite WHERE target_id = t.id), '[]'::json) as "permission_overwrites!",
    (SELECT json_agg(tag_id) FROM channel_tag WHERE channel_id = t.id) as tags,
    (SELECT json_agg(tag.*) FROM tag WHERE channel_id = t.id) as tags_available
FROM channel t
WHERE t.id = ANY($1)
