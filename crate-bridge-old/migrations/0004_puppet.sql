create table puppet (
    id uuid primary key,
    ext_platform text not null,
    ext_id text not null,
    ext_avatar text,
    name text not null,
    avatar text,
    bot boolean
);
