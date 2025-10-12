SELECT
    t.id,
    t.room_id,
    t.creator_id,
    t.owner_id,
    t.version_id,
    t.name,
    t.description,
    t.icon,
    t.type as "ty: _",
    t.nsfw,
    t.locked,
    t.archived_at,
    t.deleted_at,
    t.parent_id,
    t.position,
    t.bitrate,
    t.user_limit,
    (SELECT COUNT(*) FROM thread_member WHERE thread_id = t.id AND membership = 'Join') AS "member_count!",
    (SELECT version_id FROM message WHERE thread_id = t.id AND deleted_at IS NULL ORDER BY id DESC LIMIT 1) as last_version_id,
    (SELECT COUNT(*) FROM message WHERE thread_id = t.id AND deleted_at IS NULL) AS "message_count!",
    (SELECT json_agg(json_build_object('id', actor_id, 'type', type, 'allow', allow, 'deny', deny)) FROM permission_overwrite WHERE target_id = t.id) as "permission_overwrites!"
FROM thread t
WHERE t.id = $1