alter table invite add column max_uses int;
alter table invite add column uses int not null default 0;
alter table invite add column created_at timestamp not null default now();
alter table invite add column expires_at timestamp;
