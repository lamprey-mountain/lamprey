alter table redex add column shortname text;

update redex
set shortname = lower(substr(md5(id::text), 1, 12))
where shortname is null;

alter table redex alter column shortname set not null;

create unique index redex_shortname_idx on redex(shortname);
