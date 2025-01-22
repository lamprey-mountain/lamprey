-- FIXME: don't delete media unless it's sufficiently old
-- select id from media where not exists (select 1 from media_link where media_id = media.id);
delete from media where id not in (select media_id from media_link);
