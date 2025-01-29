create table attachment (
    chat_id text not null,
    matrix_id text not null,
    primary key (chat_id, matrix_id)
);
