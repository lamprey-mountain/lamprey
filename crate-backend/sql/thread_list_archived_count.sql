SELECT count(t.id)
FROM channel t
LEFT JOIN thread_member tm ON t.id = tm.channel_id AND tm.user_id = $2
WHERE t.parent_id = $1
  AND t.deleted_at IS NULL
  AND t.archived_at IS NOT NULL
  AND t.type IN ('ThreadPublic', 'ThreadPrivate')
  AND ($3::boolean OR t.type = 'ThreadPublic' OR (t.type = 'ThreadPrivate' AND tm.user_id IS NOT NULL AND tm.membership = 'Join'))
