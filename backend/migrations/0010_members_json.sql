CREATE OR REPLACE VIEW members_json AS (
    WITH roles_json AS (
        SELECT room_id, user_id, coalesce(json_agg(roles.*), '[]') AS roles
        FROM roles_members
        JOIN roles ON roles_members.role_id = roles.id
        GROUP BY room_id, user_id
    )
    SELECT room_members.*, row_to_json(users) AS user, roles
    FROM room_members
    JOIN users ON users.id = room_members.user_id
    JOIN roles_json ON roles_json.user_id = room_members.user_id
     AND roles_json.room_id = room_members.room_id
);
