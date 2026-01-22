SELECT
    t.id, t.room_id, t.creator_id, t.owner_id, t.version_id, t.name, t.description, t.icon, t.url, t.type as "ty: _", t.nsfw, t.locked, t.locked_until, t.locked_roles, t.archived_at, t.deleted_at, t.parent_id, t.position, t.bitrate, t.user_limit, t.invitable, t.auto_archive_duration, t.default_auto_archive_duration, t.slowmode_thread, t.slowmode_message, t.default_slowmode_message, t.last_activity_at,
    (SELECT coalesce(COUNT(*), 0) FROM thread_member WHERE channel_id = t.id AND membership = 'Join') AS "member_count!",
    (SELECT version_id FROM message WHERE channel_id = t.id AND deleted_at IS NULL ORDER BY id DESC LIMIT 1) as last_version_id,
    (SELECT coalesce(COUNT(*), 0) FROM message WHERE channel_id = t.id AND deleted_at IS NULL) AS "message_count!",
    '[]'::json as "permission_overwrites!",
    (SELECT json_agg(tag_id) FROM channel_tag WHERE channel_id = t.id) as tags,
    NULL::json as tags_available
FROM channel t
LEFT JOIN (
    SELECT channel_id, json_agg(tag_id) as json
    FROM channel_tag
    GROUP BY channel_id
) tags ON tags.channel_id = t.id
LEFT JOIN thread_member tm ON t.id = tm.channel_id AND tm.user_id = $6
WHERE t.parent_id = $5
  AND t.id > $1
  AND t.id < $2
  AND t.deleted_at IS NOT NULL
  AND t.type IN ('ThreadPublic', 'ThreadPrivate', 'ThreadForum2')
  AND ($7::boolean OR t.type IN ('ThreadPublic', 'ThreadForum2') OR (t.type = 'ThreadPrivate' AND tm.user_id IS NOT NULL AND tm.membership = 'Join'))
ORDER BY
    (CASE WHEN $3 = 'f' THEN t.id END),
    t.id DESC
LIMIT $4
