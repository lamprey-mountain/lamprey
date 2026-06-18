create table message (
    id integer primary key autoincrement,
    portal_id integer not null,
    source_platform text not null,
    source_id text not null,
    foreign key (portal_id) references portal(id)
);
