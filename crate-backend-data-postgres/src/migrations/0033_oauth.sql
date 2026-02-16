create table oauth (
    provider text not null,
    user_id uuid not null,
    remote_id text not null,
    can_auth boolean not null,
    primary key (provider, user_id),
    foreign key (user_id) REFERENCES usr(id)
);

insert into oauth (select 'discord', id, discord_id, true from usr where discord_id is not null);

alter table usr drop column discord_id;
