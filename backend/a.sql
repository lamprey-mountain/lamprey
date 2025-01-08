with last_id as (
    select thread_id, max(version_id) as last_version_id from message group by thread_id
)
select
    thread.*,
    usr.id as user_id,
    count as message_count,
    last_version_id,
    unread.version_id as last_read_id,
    coalesce(last_version_id != unread.version_id, true) as is_unread
from thread
join message_count on message_count.thread_id = thread.id
join last_id on last_id.thread_id = thread.id
full outer join usr on true
left join unread on usr.id = unread.user_id and thread.id = unread.thread_id

-- insert into unread (thread_id, user_id, version_id) values
insert into unread (thread_id, user_id, version_id) values ('01943d76-ad79-718f-9387-946138f8dfd1', '019438e7-584f-793f-8c5e-739416e011ce', '019446f3-7e2d-76e1-91b2-093bb512a2f6')
on conflict on constraint unread_pkey do update set version_id = excluded.version_id;

insert into unread (thread_id, user_id, version_id) values ('01943d76-ad79-718f-9387-946138f8dfd1', '019438e7-584f-793f-8c5e-739416e011ce', (select max(version_id) from message where thread_id = '01943d76-ad79-718f-9387-946138f8dfd1'))
on conflict on constraint unread_pkey do update set version_id = excluded.version_id;
