alter table url_embed add column thumbnail uuid;
update url_embed set thumbnail = media where media_is_thumbnail;
update url_embed set media = null where not media_is_thumbnail;
