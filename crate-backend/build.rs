use vergen_gix::{BuildBuilder, CargoBuilder, Emitter, GixBuilder, RustcBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=sql");
    println!("cargo:rerun-if-changed=build.rs");

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
