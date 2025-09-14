alter table session add column last_seen_at timestamp not null default now();
alter table session alter column last_seen_at drop default;
