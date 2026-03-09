update audit_log a set payload_prev = (
    select payload from audit_log b
    where (a.payload->'thread'->>'id' = b.payload->'thread'->>'id'
    or a.payload->'user'->>'id' = b.payload->'user'->>'id'
    or a.payload->'role'->>'id' = b.payload->'role'->>'id'
    or a.payload->'member'->>'user_id' = b.payload->'member'->>'user_id')
    and b.id < a.id
    order by id desc limit 1
);
