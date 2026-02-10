alter table config_internal add column vapid_private_key text;
alter table config_internal add column vapid_public_key text;
alter table config_internal add column oidc_jwk_key text;
alter table config_internal add column admin_token text;

update config_internal set
    vapid_private_key = value->>'vapid_private_key',
    vapid_public_key = value->>'vapid_public_key',
    oidc_jwk_key = value->>'oidc_jwk_key',
    admin_token = value->>'admin_token';

alter table config_internal alter column vapid_private_key set not null;
alter table config_internal alter column vapid_public_key set not null;
alter table config_internal alter column oidc_jwk_key set not null;

alter table config_internal drop column value;
