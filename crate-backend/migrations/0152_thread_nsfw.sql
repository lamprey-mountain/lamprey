alter table thread add column nsfw boolean not null default false;
alter table thread alter column nsfw drop default;
