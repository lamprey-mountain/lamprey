{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = [
            # Add additional build inputs here
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        craneLibLLvmTools = craneLib.overrideToolchain
          (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
          ]);

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        
        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };

        fileSetForCrate = crate: lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            (craneLib.fileset.commonCargoSources ./crate-backend)
            (craneLib.fileset.commonCargoSources crate)
          ];
        };

        # Build the top-level crates of the workspace as individual derivations.
        # This allows consumers to only depend on (and build) only what they need.
        # Though it is possible to build the entire workspace as a single derivation,
        # so this is left up to you on how to organize things
        #
        # Note that the cargo workspace must define `workspace.members` using wildcards,
        # otherwise, omitting a crate (like we do below) will result in errors since
        # cargo won't be able to find the sources for all members.
        backend = craneLib.buildPackage (individualCrateArgs // {
          pname = "backend";
          cargoExtraArgs = "-p backend";
          src = fileSetForCrate ./crate-backend;
        });
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit backend;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          # rust-clippy = craneLib.cargoClippy (commonArgs // {
          #   inherit cargoArtifacts;
          #   cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          # });

          # rust-doc = craneLib.cargoDoc (commonArgs // {
          #   inherit cargoArtifacts;
          # });

          # # Check formatting
          # rust-fmt = craneLib.cargoFmt {
          #   inherit src;
          # };

          # # Audit dependencies
          # rust-audit = craneLib.cargoAudit {
          #   inherit src advisory-db;
          # };

          # # Audit licenses
          # rust-deny = craneLib.cargoDeny {
          #   inherit src;
          # };

          # # Run tests with cargo-nextest
          # # Consider setting `doCheck = false` on `my-crate` if you do not want
          # # the tests to run twice
          # rust-nextest = craneLib.cargoNextest (commonArgs // {
          #   inherit cargoArtifacts;
          #   partitions = 1;
          #   partitionType = "count";
          #   cargoNextestPartitionsExtraArgs = "--no-tests=pass";
          # });
        };

        packages = {
          backend = backend;
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        apps.default = flake-utils.lib.mkApp {
          drv = backend;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = [ pkgs.deno ];
        };
      });
}
