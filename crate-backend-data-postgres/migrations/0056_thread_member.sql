create table thread_member (
    thread_id UUID NOT NULL,
    user_id UUID NOT NULL,
    membership TEXT NOT NULL,
    override_name TEXT,
    override_description TEXT,
    membership_changed_at timestamp not null default now(),
    foreign key (thread_id) references thread(id),
    foreign key (user_id) references usr(id),
    primary key (thread_id, user_id)
);
