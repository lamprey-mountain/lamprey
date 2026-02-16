-- https://dba.stackexchange.com/a/305274

create or replace function min(uuid, uuid)
    returns uuid
    immutable parallel safe
    language plpgsql as
$$
begin
    return least($1, $2);
end
$$;

create or replace aggregate min(uuid) (
    sfunc = min,
    stype = uuid,
    combinefunc = min,
    parallel = safe,
    sortop = operator (<)
    );

create or replace function max(uuid, uuid)
    returns uuid
    immutable parallel safe
    language plpgsql as
$$
begin
    return greatest($1, $2);
end
$$;

create or replace aggregate max(uuid) (
    sfunc = max,
    stype = uuid,
    combinefunc = max,
    parallel = safe,
    sortop = operator (>)
    );
