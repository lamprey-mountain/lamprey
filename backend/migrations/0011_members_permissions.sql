CREATE OR REPLACE VIEW member_permissions AS (
    WITH room_member_permissions AS (
        SELECT room_id, user_id, unnest(roles.permissions) AS permission
        FROM roles_members
        JOIN roles ON roles_members.role_id = roles.id
        UNION ALL
        SELECT room_id, user_id, 'View' AS permission
        FROM room_members
    )
    SELECT r.*, threads.id AS thread_id
    FROM room_member_permissions AS r
    JOIN threads ON threads.room_id = r.room_id
);
