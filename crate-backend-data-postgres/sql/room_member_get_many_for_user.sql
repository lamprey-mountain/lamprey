with r as (
    select user_id, role.room_id, array_agg(role_id) as roles from role_member
    join role on role_member.role_id = role.id
    where user_id = $1 AND role.room_id = ANY($2::uuid[])
    group by user_id, role.room_id
)
SELECT
    m.room_id,
    m.user_id,
    membership as "membership: _",
    override_name,
    override_description,
    joined_at,
    origin,
    mute,
    deaf,
    timeout_until,
    coalesce(r.roles, '{}') as "roles!",
    quarantined
FROM room_member m
left join r on r.room_id = m.room_id AND r.user_id = m.user_id
WHERE m.user_id = $1 AND m.room_id = ANY($2::uuid[]) AND m.membership = 'Join'