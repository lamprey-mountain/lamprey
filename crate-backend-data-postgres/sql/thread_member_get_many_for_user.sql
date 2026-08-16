SELECT
    channel_id,
    user_id,
    membership as "membership: _",
    joined_at
FROM thread_member
WHERE user_id = $1 AND channel_id = ANY($2::uuid[]) AND membership = 'Join'