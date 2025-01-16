-- FIXME: don't delete media unless it's sufficiently old
delete from media where id not in (select media_id from media_link);
