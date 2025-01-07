CREATE OR REPLACE VIEW room_member_permission AS (
    SELECT m.room_id, m.user_id, unnest(role.permissions) AS permission
    FROM room_member AS m
    JOIN role_member AS r ON r.user_id = m.user_id
    JOIN role ON r.role_id = role.id AND role.room_id = m.room_id
    UNION
    SELECT room_id, user_id, 'View' AS permission
    FROM room_member
);

CREATE OR REPLACE VIEW thread_member_permission AS (
    SELECT r.*, thread.id AS thread_id
    FROM room_member_permission AS r
    LEFT JOIN thread ON thread.room_id = r.room_id
);
