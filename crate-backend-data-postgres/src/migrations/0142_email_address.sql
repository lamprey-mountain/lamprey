create table user_email_addresses (
    addr text not null,
    user_id uuid not null,
    is_verified boolean not null default false,
    is_primary boolean not null default false,
    last_verification_email_sent_at timestamp not null default now(),
    primary key (user_id, addr),
    foreign key (user_id) references usr(id) on delete set null
);

create index on user_email_addresses (user_id);

create table email_address_verification (
    code text primary key,
    addr text not null,
    user_id uuid not null,
    created_at timestamp not null default now(),
    expires_at timestamp not null,
    foreign key (user_id, addr) references user_email_addresses(user_id, addr) on delete cascade
);
