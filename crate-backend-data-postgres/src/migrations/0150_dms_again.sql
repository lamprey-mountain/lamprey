alter type thread_type add value 'Dm';

create table dm (
    user_a_id uuid not null,
    user_b_id uuid not null,
    thread_id uuid not null,
    foreign key (user_a_id) references usr(id),
    foreign key (user_b_id) references usr(id),
    foreign key (thread_id) references thread(id),
    primary key (user_a_id, user_b_id),
    constraint enforce_canonical_order check(user_a_id < user_b_id)
);

alter table dm add constraint no_self_dm check (user_a_id != user_b_id);
