WITH user_roles AS (
    SELECT array_agg(rm.role_id) as roles
    FROM role_member rm
    JOIN role r ON rm.role_id = r.id
    WHERE rm.user_id = $7 AND r.room_id = $1
)
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
  AND t.deleted_at IS NULL
  AND t.archived_at IS NOT NULL
  AND ($6::uuid IS NULL OR t.parent_id = $6)
  AND NOT EXISTS (
    SELECT 1
    FROM permission_overwrite po
    WHERE (po.target_id = t.id OR po.target_id = t.parent_id)
      AND po.deny @> '"ViewThread"'::jsonb
      AND (
          (po.type = 'User' AND po.actor_id = $7)
          OR (po.type = 'Role' AND po.actor_id = t.room_id) -- @everyone
          OR (po.type = 'Role' AND po.actor_id = ANY(COALESCE((SELECT roles FROM user_roles), '{}'::uuid[])))
      )
  )
ORDER BY
    (CASE WHEN $4 = 'f' THEN t.id END),
    t.id DESC
LIMIT $5