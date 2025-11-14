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

    let build = BuildBuilder::all_build()?;
    let cargo = CargoBuilder::all_cargo()?;
    let rustc = RustcBuilder::all_rustc()?;
    let git = GixBuilder::all_git()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .add_instructions(&git)?
        .emit()?;
    Ok(())
}
