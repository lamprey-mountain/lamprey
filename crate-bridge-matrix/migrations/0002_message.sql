create table message (
    chat_id text not null,
    matrix_id text not null,
    chat_thread_id text not null,
    matrix_room_id text not null,
    primary key (chat_id, matrix_id)
);
