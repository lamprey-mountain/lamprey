CREATE VIEW hydrated_mentions AS
SELECT
    msg.id as message_id,
    jsonb_build_object(
        'users', COALESCE((SELECT jsonb_agg(jsonb_build_object('id', u.id, 'resolved_name', COALESCE(rm.override_name, u.name))) FROM jsonb_array_elements_text(msg.mentions->'users') AS uid JOIN usr u ON u.id = uid::uuid LEFT JOIN room_member rm ON rm.user_id = u.id AND rm.room_id = ch.room_id), '[]'::jsonb),
        'roles', COALESCE((SELECT jsonb_agg(jsonb_build_object('id', r.id)) FROM jsonb_array_elements_text(msg.mentions->'roles') AS rid JOIN role r ON r.id = rid::uuid), '[]'::jsonb),
        'channels', COALESCE((SELECT jsonb_agg(jsonb_build_object('id', c.id, 'room_id', c.room_id, 'type', c.type, 'name', c.name)) FROM jsonb_array_elements_text(msg.mentions->'channels') AS cid JOIN channel c ON c.id = cid::uuid), '[]'::jsonb),
        'emojis', COALESCE((SELECT jsonb_agg(jsonb_build_object('id', e.id, 'name', e.name, 'animated', e.animated)) FROM jsonb_array_elements_text(msg.mentions->'emojis') AS eid JOIN custom_emoji e ON e.id = eid::uuid), '[]'::jsonb),
        'everyone', msg.mentions->'everyone'
    ) as mentions
FROM message msg
JOIN channel ch on msg.channel_id = ch.id;