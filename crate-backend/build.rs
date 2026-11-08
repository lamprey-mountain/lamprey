use vergen_gitcl::{Cargo, Emitter, Gitcl, Rustc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cargo = Cargo::builder()
        .opt_level(true)
        .debug(true)
        .target_triple(true)
        .build();
    let git = Gitcl::builder()
        .sha(true)
        .commit_timestamp(true)
        .dirty(true)
        .build();
    let rustc = Rustc::builder()
        .semver(true)
        .llvm_version(true)
        .commit_hash(true)
        .channel(true)
        .build();

    Emitter::default()
        .add_instructions(&cargo)?
        .add_instructions(&git)?
        .add_instructions(&rustc)?
        .emit()?;

    Ok(())
}
