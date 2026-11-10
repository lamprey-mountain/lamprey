# install

at the minimum, kerosene needs:

- a database: currently only postgres
- a stream: currently you can use memory or nats + jetstream
- a blob store: currently either a local directory or an s3-compatible bucket

but you probably also want these:

- a smtp server to send email to users
- a voice sfu for voice and video support
- an opentelemetry collector for debugging

## docker

Docker is the (currently) only supported way to run kerosene. A minimal compose
file, with the backend and a caddy proxy in front of it:

```yaml
services:
  postgres:
    container_name: lamprey-postgres
    image: postgres:17
    user: "1000"
    volumes:
      - postgres:/var/lib/postgresql/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "pg_isready", "-U", "postgres"]
      interval: 10s
      start_period: 30s
    environment:
      POSTGRES_PASSWORD: changeme
  backend:
    container_name: lamprey-backend
    image: git.celery.eu.org/lamprey/lamprey-backend:latest
    restart: unless-stopped
    command: serve
    depends_on:
      - postgres
    volumes:
      - ./config-backend.toml:/config.toml:ro
  proxy:
    container_name: lamprey-proxy
    build:
      context: caddy
      dockerfile: Dockerfile
    restart: unless-stopped
    volumes:
      - ./caddy:/etc/caddy:ro # copy the Caddyfile below here
      - ./static:/static:ro # copy static files here if neeed
      - caddy_data:/data
      - caddy_config:/config
volumes:
  postgres:
  bridge:
  caddy_data:
  caddy_config:
```

`config-backend.toml` is a kerosene [config](./config.md).

Caddyfile for the above config, which serves the frontend statics and forwards
api and websocket traffic to the backend:

```caddyfile
http://example.com {
	@ws {
		method GET
		header Upgrade websocket
	}

	handle @ws {
		reverse_proxy backend:4000
	}

	handle /api/* {
		reverse_proxy backend:4000
	}

	handle {
		root /static
		try_files {path} /
		file_server
	}
}

http://example-cdn.com {
	reverse_proxy cdn:4001
}
```

You shouldn't need to serve static files separately, kerosene should have the
frontend ui assets embedded already.

## nix

TODO: write a nix module
