create type media_source as enum ('Uploaded', 'Downloaded', 'Extracted', 'Generated');
create type media_track_type as enum ('Video', 'Audio', 'Image', 'Trickplay', 'Thumbnail', 'TimedText', 'Text', 'Mixed', 'Other');

create table media_track (
    media_id uuid,
    url text not null,
    size int not null,
    mime text not null,

    source media_source not null,
    source_url text,
    
    info media_track_type not null,
    height INT,
    width INT,
    duration INT,
    codec TEXT,
    language TEXT,
    FOREIGN KEY (media_id) REFERENCES media(id)
);
