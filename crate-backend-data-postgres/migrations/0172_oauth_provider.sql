alter table application add column oauth_secret text;
alter table application add column oauth_public boolean not null default true;
alter table application add column oauth_redirect_uris jsonb not null default '[]';
alter table application alter column oauth_public drop default;
alter table application alter column oauth_redirect_uris drop default;
