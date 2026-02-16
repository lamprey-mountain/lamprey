alter table channel add column locked_until timestamp;
alter table channel add column locked_roles uuid[] not null default '{}';
