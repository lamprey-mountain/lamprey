WITH r AS (
    SELECT
        user_id,
        array_agg(role_id) AS roles
    FROM
        role_member
    JOIN role ON role.room_id = $1
        AND role_member.role_id = role.id
GROUP BY
    user_id
)
SELECT
    m.room_id,
    m.user_id,
    m.membership AS "membership: _",
    m.override_name,
    m.override_description,
    m.joined_at,
    m.origin,
    m.mute,
    m.deaf,
    m.timeout_until,
    coalesce(r.roles, '{}') AS "roles!",
    u.id as u_id,
    u.version_id as u_version_id,
    u.parent_id as u_parent_id,
    u.name as u_name,
    u.description as u_description,
    u.avatar as u_avatar,
    u.banner as u_banner,
    u.bot as u_bot,
    u.system as u_system,
    u.registered_at as u_registered_at,
    u.deleted_at as u_deleted_at,
    u.suspended as u_suspended,
    w.channel_id as "u_webhook_channel_id?",
    w.creator_id as "u_webhook_creator_id?",
    c.room_id as "u_webhook_room_id?",
    p.application_id as "u_puppet_application_id?",
    p.external_id as "u_puppet_external_id?",
    p.external_url as "u_puppet_external_url?",
    p.alias_id as "u_puppet_alias_id?"
FROM
    room_member m
    JOIN usr u ON m.user_id = u.id
    LEFT JOIN r ON r.user_id = m.user_id
    LEFT JOIN webhook w ON u.id = w.id
    LEFT JOIN channel c ON w.channel_id = c.id
    LEFT JOIN puppet p on u.id = p.id
WHERE
    m.room_id = $1
    AND m.membership = 'Join'
    AND ($2::TEXT IS NULL
        OR u.name ILIKE $2
        OR m.override_name ILIKE $2)
    AND (cardinality($4::uuid[]) = 0
        OR r.roles @> $4)
    AND ($5::TEXT IS NULL
        OR m.origin ->> 'code' = $5)
    AND ($6::BOOLEAN IS NULL
        OR (
            $6 = TRUE
            AND m.timeout_until IS NOT NULL
            AND m.timeout_until > now())
        OR (
            $6 = FALSE
            AND (m.timeout_until IS NULL
                OR m.timeout_until <= now())))
    AND ($7::BOOLEAN IS NULL
        OR m.mute = $7)
    AND ($8::BOOLEAN IS NULL
        OR m.deaf = $8)
    AND ($9::BOOLEAN IS NULL
        OR ($9 = TRUE
            AND m.override_name IS NOT NULL)
        OR ($9 = FALSE
            AND m.override_name IS NULL))
    AND ($10::BOOLEAN IS NULL
        OR ($10 = TRUE
            AND u.registered_at IS NULL)
        OR ($10 = FALSE
            AND u.registered_at IS NOT NULL))
    AND ($11::timestamp IS NULL
        OR m.joined_at < $11)
    AND ($12::timestamp IS NULL
        OR m.joined_at > $12)
    AND ($13::timestamp IS NULL
        OR extract_timestamp_from_uuid_v7(u.id) < $13)
    AND ($14::timestamp IS NULL
        OR extract_timestamp_from_uuid_v7(u.id) > $14)
ORDER BY
    u.name
LIMIT $3