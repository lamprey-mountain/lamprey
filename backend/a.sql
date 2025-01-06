select r.room_id, r.user_id, unnest(a.permissions) from room_members as r
select x.name, y.name, r.room_id, r.user_id, unnest(a.permissions) from room_members as r
join roles_members as m on m.user_id = m.user_id
join roles as a on a.id = m.role_id
join users as x on x.id = r.user_id
join rooms as y on y.id = r.room_id
union all select room_id, user_id, 'View' as permission from room_members
;

with z as (
    select id as thread_id, room_id from threads
    union all
    select null as thread_id, id as room_id from rooms
), x as (
    select r.room_id, user_id, unnest(r.permissions) as permission
    from roles_members as a
    join roles as r on a.role_id = r.id
    union all
    select room_id, user_id, 'View' AS permission
    from room_members
)
select z.room_id, user_id, thread_id, permission from x
full outer join z on z.room_id = x.room_id;

select r.room_id, r.user_id, unnest(n.permissions) from room_members as r
join roles_members as m on m.user_id = r.user_id
join roles as n on n.room_id = r.room_id and n.id = m.role_id

select x.name, y.name, t.id as thread_id, r.room_id, r.user_id, unnest(n.permissions) from room_members as r
join roles_members as m on m.user_id = r.user_id
join roles as n on n.room_id = r.room_id and n.id = m.role_id
full outer join threads as t on t.room_id = r.room_id
join users as x on x.id = r.user_id
join rooms as y on y.id = r.room_id

select user_id, role_id from roles_members;
select roles.id as role_id, roles.room_id, unnest(permissions) from roles;

select room_members.room_id, room_members.user_id, unnest(roles.permissions) from room_members
join roles_members on roles_members.user_id = room_members.user_id
join roles on roles_members.role_id = roles.user_id and roles_members.role_id = roles.user_id
;

CREATE OR REPLACE VIEW members_thread_permissions AS (
    SELECT threads.id as thread_id, user_id, unnest(roles.permissions) AS permission
    FROM roles_members
    JOIN roles ON roles_members.role_id = roles.id
    JOIN threads ON threads.room_id = roles.room_id
);

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
left JOIN threads ON threads.room_id = r.room_id
where r.room_id = '019438e8-a66b-737a-a07a-aef35c43ac9b' and r.user_id = '019438e7-584f-793f-8c5e-739416e011ce'
