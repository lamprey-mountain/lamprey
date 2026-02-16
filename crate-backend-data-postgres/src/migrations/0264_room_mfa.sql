alter table room add column security_require_mfa boolean not null default false;
alter table room alter column security_require_mfa drop default;
alter table room add column security_require_sudo boolean not null default false;
alter table room alter column security_require_sudo drop default;
