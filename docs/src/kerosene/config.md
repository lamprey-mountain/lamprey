# config

kerosene reads a single toml file (`/config.toml`, or `--config`).
Everything has a default except `rust_log`, `database_url`, `api_url`,
`html_url`, `cdn_url`, `blobs`, and `smtp`. The full config definition is in
`kerosene-core/src/config/mod.rs`, but common keys and defaults are shown below.

```toml
rust_log = "debug,sqlx=debug" # tracing EnvFilter
database_url = "postgres://user:pass@localhost:5432/chat"

api_url = "https://example.com" # public api url
html_url = "https://example.com" # public web ui url
cdn_url = "https://example-cdn.com" # public media url
# hostname = "example.com" # public hostname, required for federation

# whether registration requires a server invite
# require_server_invite = true
# max_user_emails = 50
# email_queue_workers = 5

# event bus. if unset, in-memory channels are used (single process only) (NOTE: multi process is broken anyways right now)
# [nats]
# addr = "localhost:4222"
# credentials = "/path/to/nats.creds"

# generate an admin token in the database, rotated every 5 minutes. this is
# how tooling is able to control the server
# enable_admin_token = true

# a static admin token. always valid, never rotated, and accepted regardless of
# `enable_admin_token`. do not use in production!
# admin_token = "static-token"

# defaults: api + metrics on 127.0.0.1:4000, media on 127.0.0.1:4001
# [[listen]]
# components = ["api", "metrics"] # api, media, metrics
# type = "tcp" # or "unix", which takes `path`
# address = "0.0.0.0"
# port = 4000

[blobs]
type = "s3" # or "fs", which takes `data_dir`
bucket = "chat-files"
endpoint = "https://s3.example.com"
region = "garage" # required even if the store ignores it
access_key_id = "GK12345"
secret_access_key = "12345"

# lamprey's own oauth authorization server. dynamic client registration
# (RFC 7591) is enabled by default and lets anyone register a public client;
# set `dynamic_registration = false` to disable it. `registration_token`
# optionally requires a bearer token and accepts the usual secret forms.
[oauth_server]
# dynamic_registration = true
# registration_token = "shared-secret"

# one table per provider; `kind` is "discord", "github", or "oidc"
[oauth_provider.discord]
kind = "discord"
client_id = "12345"
client_secret = "67890"
authorization_url = "https://discord.com/oauth2/authorize"
token_url = "https://discord.com/api/v10/oauth2/token"
revocation_url = "https://discord.com/api/v10/oauth2/token/revoke"
# scope = "identify email" # defaults per provider
# autoregister = false # treat new accounts from this provider as registered

[oauth_provider.keycloak]
kind = "oidc"
client_id = "lamprey"
client_secret = "67890"
authorization_url = "https://keycloak.example.com/realms/myrealm/protocol/openid-connect/auth"
token_url = "https://keycloak.example.com/realms/myrealm/protocol/openid-connect/token"
revocation_url = "https://keycloak.example.com/realms/myrealm/protocol/openid-connect/revoke"
userinfo_url = "https://keycloak.example.com/realms/myrealm/protocol/openid-connect/userinfo" # required for oidc
# scope = "openid profile email"

# outgoing http, used for url unfurling and embeds
[http]
# contact = "admin@example.com" # included in the outbound user agent
# user_agent = "my-lamprey/1.0"
deny = [ # cidrs that are never fetched (this is the default list)
  "127.0.0.1/8",
  "10.0.0.0/8",
  "172.16.0.0/12",
  "192.168.0.0/16",
  "100.64.0.0/10",
  "169.254.0.0/16",
  "::1/128",
  "fe80::/64",
  "fc00::/7",
]
# max_parallel_jobs = 5

# outgoing mail, used for magic links and password resets
[smtp]
username = "system@example.com"
password = "a1b2c3"
host = "mail.example.com"
from = '"system" <system@example.com>'

[media]
max_size = 16777216 # bytes, default 8 MiB
thumb_sizes = [64, 320, 640] # in pixels; this is the default list and generally what most code expects
# cache_media = 10000 # entries
# cache_emoji = 1000000 # entries

# external services that score uploads; see also `moderation.automod_media`
# [[media.scanners]]
# key = "nsfw"
# version = 1
# scan_url = "http://scanner-nsfw:8080/scan"
# health_url = "http://scanner-nsfw:8080/health"

# omit this table to disable voice. the address overrides are what the voice
# server binds to and announces to clients
[voice]
token = "a1b2c3" # shared secret with crate-voice
host_ipv4 = "192.168.12.34" # address to bind and announce, must be reachable
host_ipv6 = "2001:db8:0a0b:12f0:0000:0000" # same, for ipv6
# workers = 4 # defaults to cpu cores
# rtp_port_min = 56000 # first rtp media udp port
# rtp_port_max = 56999 # last rtp media udp port; range must be >= workers wide
# quic_port = 0 # defaults to a random port
# stun_port = 3478 # builtin stun server, disabled by default
#
# the rtp range must be reachable from clients (open it in your firewall/nat)
# and each shard binds one port from it.

[webtransport]
# webtransport should just work but you can manually configure it like so
# address = "0.0.0.0"
# port = 4433
# url = "https://example.com:4433/api/v1/sync-webtransport" # defaults to `api_url`

# otlp exporters; unset endpoints fall back to OTEL_EXPORTER_OTLP_* env vars
# [otel]
# enabled = false
# logs = "http://localhost:4317"
# traces = "http://localhost:4317"
# metrics = "http://localhost:4317"

# overrides for ui serving. defaults to the embedded ui, if it exists
# [ui]
# path = "/srv/lamprey/static"

# path to a maxmind geoip database with country info, if you have one
# mmdb_path = "/var/lib/GeoLite2-Country.mmdb"
```
