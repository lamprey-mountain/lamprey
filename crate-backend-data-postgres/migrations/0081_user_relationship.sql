create type user_relationship_type as ENUM ('Friend', 'Outgoing', 'Incoming', 'Block');

create table user_relationship (
    user_id uuid not null,
    other_id uuid not null,
    rel user_relationship_type,
    note text,
    petname text,
    ignore_forever boolean,
    ignore_until timestamp,
    foreign key (user_id) references usr(id),
    foreign key (other_id) references usr(id),
    primary key (user_id, other_id)
);
