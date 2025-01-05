CREATE OR REPLACE VIEW members_permissions AS (
    SELECT room_id, user_id, unnest(roles.permissions) AS permission
    FROM roles_members
    JOIN roles ON roles_members.role_id = roles.id
    UNION ALL
    SELECT room_id, user_id, 'View' AS permission
    FROM room_members
);
