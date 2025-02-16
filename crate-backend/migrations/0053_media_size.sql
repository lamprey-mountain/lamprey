create type media_size_type as enum ('Bytes', 'BytesPerSecond');
alter table media_track add column size_type media_size_type not null default 'Bytes';
alter table media_track alter column size_type drop default;
