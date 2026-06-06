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
),
direct_counts AS (
    SELECT mv.reply_id as id, count(*) as count_direct
    FROM message m
    JOIN message_version mv ON m.latest_version_id = mv.version_id
    WHERE m.channel_id = $1 AND m.deleted_at IS NULL AND mv.reply_id IS NOT NULL
    GROUP BY mv.reply_id
),
recursive_counts_cte AS (
    SELECT mv.reply_id, m.id as descendant_id
    FROM message m
    JOIN message_version mv ON m.latest_version_id = mv.version_id
    WHERE m.channel_id = $1 AND m.deleted_at IS NULL AND mv.reply_id IS NOT NULL
    UNION ALL
    SELECT rc.reply_id, m.id
    FROM recursive_counts_cte rc
    JOIN message_version mv ON mv.reply_id = rc.descendant_id
    JOIN message m ON m.latest_version_id = mv.version_id
    WHERE m.channel_id = $1 AND m.deleted_at IS NULL
),
recursive_counts AS (
    SELECT reply_id as id, count(*) as count_recursive
    FROM recursive_counts_cte
    GROUP BY reply_id
)
SELECT
    mv.type as "message_type: DbMessageType",
    m.id,
    m.channel_id,
    c.room_id,
    m.author_id,
    m.created_at,
    m.deleted_at,
    m.removed_at,
    m.pinned,
    m.created_seq,
    m.lifecycle_seq,
    m.flume,
    m.interaction,
    m.ephemeral,
    mv.version_id,
    mv.author_id as version_author_id,
    mv.content,
    mv.metadata,
    mv.reply_id,
    mv.override_name,
    mv.embeds as "embeds",
    mv.components as "components",
    mv.created_at as version_created_at,
    mv.deleted_at as version_deleted_at,
    mv.created_seq as version_created_seq,
    coalesce(att_json.attachments, '{}') as "attachments!",
    coalesce(dc.count_direct, 0) as count_direct,
    coalesce(rc.count_recursive, 0) as count_recursive
FROM message AS m
JOIN filtered_messages fm ON m.id = fm.id
JOIN message_version AS mv ON m.latest_version_id = mv.version_id
LEFT JOIN att_json ON att_json.version_id = mv.version_id
JOIN channel AS c ON m.channel_id = c.id
LEFT JOIN direct_counts dc ON m.id = dc.id
LEFT JOIN recursive_counts rc ON m.id = rc.id
WHERE m.channel_id = $1 AND m.deleted_at IS NULL
  AND m.id > $5 AND m.id < $6
ORDER BY (CASE WHEN $7 = 'f' THEN m.id END), m.id DESC LIMIT $8
