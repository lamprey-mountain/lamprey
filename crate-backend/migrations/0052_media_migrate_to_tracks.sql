insert into media_track (
    media_id, url, size, mime,
    source, source_url,
    info, height, width, duration, codec, language
)
select
    id, url, size, mime,
    'Uploaded', null,
    'Mixed', height, width, duration, null, null
from media;

alter table media drop column duration;
alter table media drop column height;
alter table media drop column width;
alter table media drop column mime;
alter table media drop column size;
alter table media drop column thumbnail_url;
alter table media drop column source_url;
alter table media drop column url;
