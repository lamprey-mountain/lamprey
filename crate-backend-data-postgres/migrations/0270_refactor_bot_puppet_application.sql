-- 1. usr.bot
alter table usr add column bot boolean not null default false;
update usr set bot = true where id in (select id from application);

-- 2. puppet
alter table puppet add column application_id uuid references application(id) on delete cascade;
update puppet set application_id = u.parent_id from usr u where u.id = puppet.id;
alter table puppet drop column external_platform;

-- 3. application_bridge
create table application_bridge (
    application_id uuid primary key references application(id) on delete cascade,
    platform_name TEXT,
    platform_url TEXT,
    platform_description TEXT
);

insert into application_bridge (application_id)
select id from application where bridge = true;

alter table application drop column bridge;
