alter table session add column expires_at timestamp;

create table oauth_authorization_code (
    code text primary key,
    application_id uuid not null references application(id) on delete cascade,
    user_id uuid not null references usr(id) on delete cascade,
    redirect_uri text not null,
    scopes jsonb not null,
    expires_at timestamp not null
);
