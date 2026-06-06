WITH RECURSIVE ancestors_cte AS (
    SELECT
        m.id,
        mv.reply_id,
        1 as depth
    FROM message m
    JOIN message_version mv ON m.latest_version_id = mv.version_id
    WHERE m.id = $1
    UNION ALL
    SELECT
        m.id,
        mv.reply_id,
        a.depth + 1
    FROM message m
    JOIN message_version mv ON m.latest_version_id = mv.version_id
    JOIN ancestors_cte a ON m.id = a.reply_id
    WHERE a.depth < $2
)
SELECT
    mv.type as "message_type: DbMessageType",
    m.id as "id!",
    m.channel_id as "channel_id!",
    c.room_id,
    m.author_id as "author_id!",
    m.created_at as "created_at!",
    m.deleted_at,
    m.removed_at,
    m.pinned,
    m.created_seq as "created_seq!",
    m.lifecycle_seq as "lifecycle_seq!",
    m.flume,
    m.interaction,
    m.ephemeral as "ephemeral!",
    mv.version_id as "version_id!",
    mv.author_id as "version_author_id!",
    mv.content,
    mv.metadata,
    mv.reply_id,
    mv.override_name,
    mv.embeds as "embeds",
    mv.components as "components",
    mv.created_at as "version_created_at!",
    mv.deleted_at as "version_deleted_at",
    mv.created_seq as "version_created_seq!",
    coalesce(att_json.attachments, '{}') as "attachments!",
    (SELECT count(*) FROM message r JOIN message_version mv_r ON r.latest_version_id = mv_r.version_id WHERE mv_r.reply_id = m.id AND r.deleted_at IS NULL)::bigint as count_direct,
    (
        WITH RECURSIVE rc AS (
            SELECT id FROM message r JOIN message_version mv_r ON r.latest_version_id = mv_r.version_id WHERE mv_r.reply_id = m.id AND r.deleted_at IS NULL
            UNION ALL
            SELECT r.id FROM rc JOIN message_version mv_r ON mv_r.reply_id = rc.id JOIN message r ON r.latest_version_id = mv_r.version_id WHERE r.deleted_at IS NULL
        ) SELECT count(*) FROM rc
    )::bigint as count_recursive
FROM ancestors_cte AS a
JOIN message AS m ON a.id = m.id
JOIN message_version AS mv ON m.latest_version_id = mv.version_id
LEFT JOIN att_json ON att_json.version_id = mv.version_id
JOIN channel AS c ON m.channel_id = c.id
ORDER BY depth DESC
