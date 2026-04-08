in general:
- prioritize correctness over hacky code
- read additional files for context if needed

## useful commands

- typecheck frontend: `pnpm tsc`
- lint frontend: `pnpm check`
- format/autofix frontend: `pnpm fix`

## additional commands

this is for reference, don't run these. for various reasons, these may be buggy or require extra work to run. wait for and tell the user to run these instead if needed.

- `deno task hakari` regenerate crate-hakari, to keep feature flags in sync across all deps
- `deno task sqlx-prep` regenerate sqlx offline
- `pnpm -F frontend test` runs playwright tests

## frontend

- uses `@/*` path alias for `frontend/src/*`. older code may use `../../foo/bar` relative imports. rewrite these if you're updating the import, but otherwise don't touch them.
- written in solidjs with tsx, styling is done with scss
- always typecheck after you're done editing

don't bother with `pnpm check` right now; there is too much noise from unfixed
lints. however, `biome check frontend/src/path/to/file.ext` should be done for
every file you have edited. in this case, fix any lints relevant to your change
as well as any trivially fixable lints, but ignore everything else.

## backend

- rust crates are named `lamprey-foo` not `crate-foo`
- always use `-p lamprey-foo` to only check/clippy/fix the files that are needed
- cargo check after changes to verify they are correct
- only cargo clippy/fix when requested
- some crates are planned but dont exist yet (`crate-backend-rest`, `crate-backend-sync`)
