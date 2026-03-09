drop table reaction_message_custom;
drop table reaction_thread_custom;
drop table reaction_message_unicode;
drop table reaction_thread_unicode;

create table reaction (
    message_id uuid not null,
    user_id uuid not null,
    reaction_key text not null,
    emoji_id uuid,
    primary key (message_id, user_id, reaction_key),
    foreign key (emoji_id) references custom_emoji(id)
);
