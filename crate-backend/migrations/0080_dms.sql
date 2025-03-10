alter type room_type add value 'Dm';

create table dm (
    user_a_id uuid not null,
    user_b_id uuid not null,
    room_id uuid not null,
    foreign key (user_a_id) references usr(id),
    foreign key (user_b_id) references usr(id),
    foreign key (room_id) references room(id),
    primary key (user_a_id, user_b_id),
    constraint enforce_canonical_order check(user_a_id < user_b_id)
);
