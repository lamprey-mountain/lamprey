update media_track
set url = 'https://chat-files.celery.eu.org/' || url
where url not like 'https://chat-files.celery.eu.org/%';
