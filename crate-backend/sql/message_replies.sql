WITH recursive message_tree AS (
    SELECT
        m.id,
        mv.reply_id,
        1 AS depth
    FROM
        message m
        JOIN message_version mv ON m.latest_version_id = mv.version_id
    WHERE
        ($2::uuid IS NOT NULL AND m.id = $2::uuid)
        OR ($2::uuid IS NULL AND mv.reply_id IS NULL)
    UNION ALL
    SELECT
        m.id,
        mv2.reply_id,
        mt.depth + 1
    FROM
        message m
        JOIN message_version mv2 ON m.latest_version_id = mv2.version_id
        JOIN message_tree mt ON mv2.reply_id = mt.id
    WHERE
        mt.depth < $3
),
ranked_messages AS (
    SELECT
        id,
        depth,
        row_number() OVER (PARTITION BY reply_id ORDER BY id) AS rn
    FROM
        message_tree
),
filtered_messages AS (
    SELECT id
    FROM ranked_messages
    WHERE (depth = 1 OR rn <= $4 OR $4 IS NULL)
)
SELECT
    mv.type as "message_type: DbMessageType",
    m.id,
    m.channel_id,
    m.author_id,
    m.created_at,
    m.deleted_at,
    m.removed_at,
    m.pinned,
    mv.version_id,
    mv.author_id as version_author_id,
    mv.content,
    mv.metadata,
    mv.reply_id,
    mv.override_name,
    mv.embeds as "embeds",
    mv.created_at as version_created_at,
    mv.deleted_at as version_deleted_at,
    coalesce(att_json.attachments, '{}') as "attachments!"
FROM message AS m
JOIN filtered_messages fm ON m.id = fm.id
JOIN message_version AS mv ON m.latest_version_id = mv.version_id
LEFT JOIN att_json ON att_json.version_id = mv.version_id
WHERE m.channel_id = $1 AND m.deleted_at IS NULL
  AND m.id > $5 AND m.id < $6
ORDER BY (CASE WHEN $7 = 'f' THEN m.id END), m.id DESC LIMIT $8
