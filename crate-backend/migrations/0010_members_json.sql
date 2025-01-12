CREATE VIEW member_json AS (
    WITH role_json AS (
        SELECT room_id, user_id, json_agg(role.*) AS roles
        FROM role_member
        JOIN role ON role_member.role_id = role.id
        GROUP BY room_id, user_id
    )
    SELECT room_member.*, row_to_json(usr) AS user, coalesce(roles, '[]') as roles
    FROM room_member
    JOIN usr ON usr.id = room_member.user_id
    LEFT JOIN role_json ON role_json.user_id = room_member.user_id
     AND role_json.room_id = room_member.room_id
);
