update media_track set mime = regexp_replace(regexp_replace(mime, E'^[\n\r ]+', '', 'g'), E'[\n\r ]+$', '', 'g');
