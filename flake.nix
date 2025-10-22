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
        src = ./.;
        common = {
          inherit src;
          strictDeps = true;
          doCheck = false;

          buildInputs = with pkgs; [
            openssl
            pkg-config
            # Add additional build inputs here
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          nativeBuildInputs = with pkgs; [
            perl
          ];
        };

        cargoArtifacts =
          craneLib.buildDepsOnly (common // { name = "(shared deps)"; });

        mkCrate = name:
          craneLib.buildPackage (common // {
            inherit cargoArtifacts;
            pname = name;
            cargoExtraArgs = "-p ${name}";
            env = {
              VERGEN_GIT_SHA = self.rev or self.dirtyRev;
            };
          });

        backend = mkCrate "lamprey-backend";
        bridge = mkCrate "lamprey-bridge";
        voice = mkCrate "lamprey-voice";
        media = mkCrate "lamprey-media";

        frontend = pkgs.stdenvNoCC.mkDerivation (finalAttrs: rec {
          name = "frontend";
          pname = name;
          src = ./.;
          version = "0.0.0";

          nativeBuildInputs = with pkgs; [ nodejs pnpm.configHook git ];

          VITE_GIT_SHA = self.rev or self.dirtyRev or "unknown";
          VITE_GIT_DIRTY = if (self ? rev) then "false" else "true";

          pnpmDepsHash = "sha256-NgoJEHUBENjPLlr/Hpt6HZrEV16FUClTIUPbNSl2xTI=";
          pnpmDeps = pkgs.pnpm.fetchDeps {
            inherit (finalAttrs) src pname version;
            fetcherVersion = 2;
            hash = pnpmDepsHash;
          };

          buildPhase = ''
            cd frontend
            pnpm run build
            mv dist $out
          '';
        });
      in {
        packages = rec {
          inherit backend bridge voice media frontend;

          cargo-deps = cargoArtifacts;

          backend-oci = pkgs.dockerTools.streamLayeredImage {
            name = "backend";
            tag = "latest";
            contents =
              [ pkgs.dockerTools.caCertificates pkgs.ffmpeg-headless pkgs.file ];
            config = {
              Entrypoint =
                [ "${pkgs.tini}/bin/tini" "--" "${backend}/bin/backend" ];
            };
          };

          bridge-oci = pkgs.dockerTools.streamLayeredImage {
            name = "bridge";
            tag = "latest";
            contents = [ pkgs.dockerTools.caCertificates ];
            config = {
              Entrypoint = [
                "${pkgs.tini}/bin/tini"
                "--"
                "${bridge}/bin/bridge"
              ];
            };
          };

          voice-oci = pkgs.dockerTools.streamLayeredImage {
            name = "voice";
            tag = "latest";
            contents = [ pkgs.dockerTools.caCertificates ];
            config = {
              Entrypoint = [
                "${pkgs.tini}/bin/tini"
                "--"
                "${voice}/bin/voice"
              ];
            };
          };

          media-oci = pkgs.dockerTools.streamLayeredImage {
            name = "media";
            tag = "latest";
            contents = [ pkgs.dockerTools.caCertificates ];
            config = {
              Entrypoint = [
                "${pkgs.tini}/bin/tini"
                "--"
                "${media}/bin/media"
              ];
            };
          };
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          # checks = self.checks.${system};
        };
      });
}
