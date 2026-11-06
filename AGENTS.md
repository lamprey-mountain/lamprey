this project contains rust (tokio + axum) and typescript (tsx + solidjs)
crates in directories starting with `crate-` like `crate-foo` are actually named `lamprey-foo`. other directory names match the crate name.
when using cargo check or test, always use `-p` to only check/test what you need
frontend uses `@/*` as a path alias for `frontend/src/*`
ask before adding dependencies or executing any git commands.
keep all mods at the top of each file, followed types (structs, enums, etc), impls, and finally fns. order everything by importance, more important and meaningful stuff earlier.
