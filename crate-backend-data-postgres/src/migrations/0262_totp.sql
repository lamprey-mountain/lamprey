alter table usr add column totp_secret text;
alter table usr add column totp_enabled boolean not null default false;
alter table usr alter column totp_enabled drop default;
