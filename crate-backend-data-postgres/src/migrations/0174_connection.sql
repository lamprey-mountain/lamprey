create table connection (
    user_id uuid,
    application_id uuid,
    sccopes jsonb not null,
    created_at timestamp not null,
    primary key (user_id, application_id),
    foreign key (user_id) references usr (id),
    foreign key (application_id) references application (id)
);
