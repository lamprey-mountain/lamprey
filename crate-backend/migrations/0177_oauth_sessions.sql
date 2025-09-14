alter table session add column type text not null default 'User';
alter table session alter column type drop default;
alter table session add column application_id uuid references application(id) on delete cascade;
