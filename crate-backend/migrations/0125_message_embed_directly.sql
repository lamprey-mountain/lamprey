alter table message add column embeds jsonb;
update message m set embeds = to_jsonb(u.embeds) from (select * from url_embed_json) u where u.version_id = m.version_id;
drop view url_embed_json;
drop table url_embed_message;
drop table url_embed;
