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
    (SELECT count(*) FROM message r JOIN message_version mv_r ON r.latest_version_id = mv_r.version_id WHERE mv_r.reply_id = m.id AND r.deleted_at IS NULL) as count_direct,
    (
        WITH RECURSIVE rc AS (
            SELECT id FROM message r JOIN message_version mv_r ON r.latest_version_id = mv_r.version_id WHERE mv_r.reply_id = m.id AND r.deleted_at IS NULL
            UNION ALL
            SELECT r.id FROM rc JOIN message_version mv_r ON mv_r.reply_id = rc.id JOIN message r ON r.latest_version_id = mv_r.version_id WHERE r.deleted_at IS NULL
        ) SELECT count(*) FROM rc
    ) as count_recursive
FROM message AS m
JOIN message_version AS mv ON m.latest_version_id = mv.version_id
LEFT JOIN att_json ON att_json.version_id = mv.version_id
JOIN channel AS c ON m.channel_id = c.id
WHERE m.channel_id = $1 AND m.id = $2 AND m.deleted_at IS NULL
