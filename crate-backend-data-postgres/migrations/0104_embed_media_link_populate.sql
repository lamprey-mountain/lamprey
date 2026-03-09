insert into media_link select id as target_id, media as media_id, 'Embed' as link_type from url_embed where media is not null;
