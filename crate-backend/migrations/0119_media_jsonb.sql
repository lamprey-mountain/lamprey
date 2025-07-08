alter table media add column data jsonb;

update media
set data = mj.json
from (select id, row_to_json(m) as json from media_json m) mj
where media.id = mj.id;

drop view if exists url_embed_json;
drop view if exists att_json;
drop view if exists media_json;

alter table media drop column filename, drop column alt;
drop table media_track;
