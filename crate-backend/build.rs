use vergen_gix::{BuildBuilder, CargoBuilder, Emitter, GixBuilder, RustcBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=sql");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_FEATURE_EMBED_FRONTEND").is_ok() {
        println!("cargo:rerun-if-env-changed=FRONTEND_DIST");
        let frontend_dist =
            std::env::var("FRONTEND_DIST").unwrap_or_else(|_| "../frontend/dist".to_string());
        println!("cargo:rustc-env=RUST_EMBED_FRONTEND_PATH={}", frontend_dist);
    }

    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let cargo = CargoBuilder::default().opt_level(true).build()?;
    let git = GixBuilder::default().commit_timestamp(true).build()?;
    let rustc = RustcBuilder::default().semver(true).build()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&git)?
        .add_instructions(&rustc)?
        .emit()?;

    Ok(())
}
