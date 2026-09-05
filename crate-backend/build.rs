use vergen_gix::{CargoBuilder, Emitter, GixBuilder, RustcBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CARGO_FEATURE_EMBED_FRONTEND").is_ok() {
        println!("cargo:rerun-if-env-changed=FRONTEND_DIST");
        let frontend_dist =
            std::env::var("FRONTEND_DIST").unwrap_or_else(|_| "../frontend/dist".to_string());
        println!("cargo:rustc-env=RUST_EMBED_FRONTEND_PATH={}", frontend_dist);
    }

    let cargo = CargoBuilder::default()
        .opt_level(true)
        .debug(true)
        .target_triple(true)
        .build()?;
    let git = GixBuilder::default()
        .commit_timestamp(true)
        .sha(true)
        .build()?;
    let rustc = RustcBuilder::default()
        .semver(true)
        .llvm_version(true)
        .commit_hash(true)
        .channel(true)
        .build()?;

    Emitter::default()
        .add_instructions(&cargo)?
        .add_instructions(&git)?
        .add_instructions(&rustc)?
        .emit()?;

    Ok(())
}
