create table url_embed (
    id uuid primary key,
    url text not null,
    canonical_url text,
    title text,
    description text,
    color text,
    media uuid,
    media_is_thumbnail boolean,
    author_url text,
    author_name text,
    author_avatar uuid,
    site_name text,
    site_avatar uuid,
    created_at timestamp not null default now(),
    foreign key (media) references media(id),
    foreign key (author_avatar) references media(id),
    foreign key (site_avatar) references media(id)
);
