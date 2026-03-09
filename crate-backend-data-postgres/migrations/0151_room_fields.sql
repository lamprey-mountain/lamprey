alter table room add column archived_at timestamp;
alter table room add column public boolean not null default false;
alter table room alter column public drop default;
