{
  inputs = {
    crane.url = "github:ipetkov/crane?ref=master";
    flake-utils.url = "github:numtide/flake-utils?ref=main";
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";
  };
  outputs = { self, crane, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        craneLib = crane.mkLib pkgs;
        # src = craneLib.cleanCargoSource ./.;
        src = ./.;
        common = {
          inherit src;
          strictDept = true;
          doCheck = false;
        };

        cargoArtifacts = craneLib.buildDepsOnly common;
        mkCrate = name: craneLib.buildPackage (common // {
          pname = name;
          cargoExtraArgs = "-p ${name}";
        });
        
        mkImage = pkg: pkgs.dockerTools.buildImage {
          name = pkg.pname;
          tag = "latest";
          copyToRoot = [ pkgs.dockerTools.caCertificates ];
          config = {
            Entrypoint = ["${pkgs.tini}/bin/tini" "--" "${pkg}"];
          };
        };
        
        backend = mkCrate "backend";
        bridge-discord = mkCrate "bridge-discord";

        # FIXME
        # frontend = pkgs.stdenvNoCC.mkDerivation (finalAttrs: rec {
        #   name = "frontend";
        #   pname = name;
        #   src = ./.;
          
        #   nativeBuildInputs = [ pkgs.nodejs pkgs.pnpm.configHook ];
          
        #   pnpmDepsHash = "sha256-woA5C1airy7eKbk3EP7cggldNFpz+9y68A16QkGrmeA=";
        #   pnpmDeps = pkgs.pnpm.fetchDeps {
        #     inherit (finalAttrs) src pname;
        #     hash = pnpmDepsHash;
        #   };
          
        #   postBuild = ''
        #     ls $src
        #     cd $src/frontend
        #     pnpm build
        #     # mv dist $out
        #   '';
        # });
      in {
        packages = rec {
          # inherit backend bridge-discord frontend;
          inherit backend bridge-discord;
          backend-oci = mkImage backend;
          bridge-discord-oci = mkImage bridge-discord;
        };
      });
}
