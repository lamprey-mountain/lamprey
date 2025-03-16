create table reaction_thread_unicode (
    thread_id uuid not null,
    user_id uuid not null,
    reaction_key text not null,
    primary key (thread_id, user_id, reaction_key),
    foreign key (thread_id) references thread(id) on delete cascade
);
