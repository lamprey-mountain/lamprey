create table channel_document (
    channel_id uuid primary key references channel(id) on delete cascade,
    draft boolean not null default false,
    archived_at timestamp,
    archived_reason text,
    template boolean not null default false,
    slug text,
    published_at timestamp,
    published_revision text,
    published_unlisted boolean
);

create table channel_wiki (
    channel_id uuid primary key references channel(id) on delete cascade,
    allow_indexing boolean not null default false,
    page_index uuid references channel(id) on delete set null,
    page_notfound uuid references channel(id) on delete set null
);

create table channel_calendar (
    channel_id uuid primary key references channel(id) on delete cascade,
    color text,
    default_timezone text not null default 'UTC'
);

insert into channel_document (channel_id)
select id from channel where type = 'Document';

insert into channel_wiki (channel_id)
select id from channel where type = 'Wiki';

insert into channel_calendar (channel_id)
select id from channel where type = 'Calendar';
