alter table thread add column position int not null default 0;
alter table thread add column parent_id uuid references thread (id);
alter table thread alter column position drop default;
