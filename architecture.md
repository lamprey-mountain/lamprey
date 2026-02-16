# architecture

how lamprey mountain is architectured

## server crates

- `crate-backend` the main backend monolith (todo: remove?)
- `crate-backend-core` core types/traits for backend
- `crate-backend-rest` rest api implementation
- `crate-backend-sync` websocket sync api implementation
- `crate-backend-data-postgres` postgres implementation
- `crate-common` common types used everywhere
- `crate-media` media proxy
- `crate-voice` voice sfu implementation

## other crates

- `crate-bot` a basic bot
- `crate-bridge` discord bridge
- `crate-hakari` used for cargo hakari
- `crate-macros` proc macros
- `crate-sdk` wip sdk for developing on lamprey

## non-rust

- `docs` very incomplete documentation
- `frontend` solidjs frontend implementation
- `scripts` random maintenance scripts (TODO: clean up)
- `tests` a sad attempt at writing some tests (TODO: actually add some more
  tests)
