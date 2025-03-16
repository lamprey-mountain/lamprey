create table reaction_thread_custom (
    thread_id uuid not null,
    user_id uuid not null,
    reaction_key uuid not null,
    primary key (thread_id, user_id, reaction_key),
    foreign key (thread_id) references thread(id) on delete cascade,
    foreign key (reaction_key) references custom_emoji(id)
);

create table reaction_message_unicode (
    message_id uuid not null,
    user_id uuid not null,
    reaction_key text not null,
    primary key (message_id, user_id, reaction_key)
);

create table reaction_message_custom (
    message_id uuid not null,
    user_id uuid not null,
    reaction_key uuid not null,
    primary key (message_id, user_id, reaction_key),
    foreign key (reaction_key) references custom_emoji(id)
);
