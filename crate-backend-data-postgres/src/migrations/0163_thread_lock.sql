alter table thread add column locked boolean not null default false;
alter table thread alter column locked drop default;
