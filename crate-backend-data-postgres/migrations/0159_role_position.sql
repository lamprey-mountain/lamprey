alter table role add column position int not null default 0;

with ranked_roles as (
    select
        id,
        row_number() over (partition by room_id order by id desc) as rn
    from role
    where id != room_id
)
update role
set position = ranked_roles.rn
from ranked_roles
where role.id = ranked_roles.id;

alter table role alter column position drop default;
