alter table url_embed_message add column ordering int not null default 0;
alter table url_embed_message alter column ordering drop default;
