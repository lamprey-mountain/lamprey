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
    (SELECT coalesce(COUNT(*), 0) FROM thread_member WHERE thread_id = t.id AND membership = 'Join') AS "member_count!",
    (SELECT version_id FROM message WHERE thread_id = t.id AND deleted_at IS NULL ORDER BY id DESC LIMIT 1) as last_version_id,
    (SELECT coalesce(COUNT(*), 0) FROM message WHERE thread_id = t.id AND deleted_at IS NULL) AS "message_count!",
    coalesce((SELECT json_agg(json_build_object('id', actor_id, 'type', type, 'allow', allow, 'deny', deny)) FROM permission_overwrite WHERE target_id = t.id), '[]'::json) as "permission_overwrites!"
FROM thread t
WHERE t.room_id = $1
  AND t.id > $2
  AND t.id < $3
  AND t.deleted_at IS NOT NULL
  AND ($6::uuid IS NULL OR t.parent_id = $6)
ORDER BY
    (CASE WHEN $4 = 'f' THEN t.id END),
    t.id DESC
LIMIT $5
