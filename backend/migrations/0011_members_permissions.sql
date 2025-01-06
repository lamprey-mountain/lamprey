CREATE OR REPLACE VIEW room_member_permissions AS (
    SELECT m.room_id, m.user_id, unnest(roles.permissions) AS permission
    FROM room_members AS m
    JOIN roles_members AS r ON r.user_id = m.user_id
    JOIN roles ON r.role_id = roles.id
    UNION
    SELECT room_id, user_id, 'View' AS permission
    FROM room_members
);

CREATE OR REPLACE VIEW thread_member_permissions AS (
    SELECT r.*, threads.id AS thread_id
    FROM room_member_permissions AS r
    LEFT JOIN threads ON threads.room_id = r.room_id
);
