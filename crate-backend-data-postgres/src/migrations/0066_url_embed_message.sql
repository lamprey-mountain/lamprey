create table url_embed_message (
    version_id UUID,
    embed_id UUID,
    foreign key (version_id) references message (version_id),
    foreign key (embed_id) references url_embed (id),
    primary key (version_id, embed_id)
);
