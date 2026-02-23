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

        backend = craneLib.buildPackage (common // {
            inherit cargoArtifacts;
            pname = "lamprey-backend";
            cargoExtraArgs = "-p lamprey-backend --features lamprey-backend/embed-frontend";
            env = {
              VERGEN_GIT_SHA = self.rev or self.dirtyRev;
              FRONTEND_DIST = frontend;
            };
        });
        bridge = mkCrate "lamprey-bridge";
        voice = mkCrate "lamprey-voice";
        media = mkCrate "lamprey-media";
        scanner-malware = mkCrate "scanner-malware";

        frontend = pkgs.stdenvNoCC.mkDerivation (finalAttrs: rec {
          name = "frontend";
          pname = name;
          src = ./.;
          version = "0.0.0";

          nativeBuildInputs = with pkgs; [ nodejs pnpm.configHook git ];

          VITE_GIT_SHA = self.rev or self.dirtyRev or "unknown";
          VITE_GIT_DIRTY = if (self ? rev) then "false" else "true";

          pnpmDepsHash = "sha256-vJQ2MruYHWcHVihYBJ1X+UZjWjhys7LTun7PQMgOPWk=";
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

        python-deps = ps: with ps; [
          fastapi
          uvicorn
          python-multipart
          torch
          transformers
          pillow
        ];

        python-env = pkgs.python3.withPackages python-deps;
      in {
        packages = rec {
          inherit backend bridge voice media frontend scanner-malware;

          scanner-nsfw = pkgs.writeShellApplication {
            name = "run-scanner-nsfw";
            runtimeInputs = [ python-env ];
            text = ''
              cd ${./scanner-nsfw}
              uvicorn app:app --host 0.0.0.0 --port 4100
            '';
          };

          cargo-deps = cargoArtifacts;

          backend-oci = pkgs.dockerTools.streamLayeredImage {
            name = "backend";
            tag = "latest";
            contents =
              [ pkgs.dockerTools.caCertificates pkgs.ffmpeg-headless pkgs.file ];
            config = {
              Entrypoint =
                [ "${pkgs.tini}/bin/tini" "--" "${backend}/bin/lamprey" ];
              Healthcheck = {
                Test = [ "CMD-SHELL" "curl -f http://localhost:8080/api/v1/health || exit 1" ];
                Interval = 30000000000; # 30s
                Timeout = 10000000000; # 10s
                Retries = 3;
                StartPeriod = 5000000000; # 5s
              };
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

          scanner-malware-oci =
            let
              scannerMalwareConfig = pkgs.writeTextFile {
                name = "scanner-malware-config";
                destination = "/etc/scanner-malware.toml";
                text = ''
                  rust_log = "info"
                  listen = { address = "0.0.0.0", port = 4101 }

                  [clamav]
                  type = "local"
                  socket = "/run/clamav/clamd.sock"
                  pid_file = "/run/clamav/clamd.pid"
                  database_directory = "/var/lib/clamav"
                  database_mirror = "database.clamav.net"
                '';
              };
            in
            pkgs.dockerTools.streamLayeredImage {
              name = "scanner-malware";
              tag = "latest";
              contents = [
                pkgs.dockerTools.caCertificates
                pkgs.clamav
                scannerMalwareConfig
              ];
              config = {
                WorkingDir = "/";
                Entrypoint = [
                  "${pkgs.tini}/bin/tini"
                  "--"
                  "${scanner-malware}/bin/scanner-malware"
                  "--config"
                  "/etc/scanner-malware.toml"
                ];
                Healthcheck = {
                  Test = [ "CMD-SHELL" "curl -f http://localhost:4101/health || exit 1" ];
                  Interval = 30000000000; # 30s
                  Timeout = 10000000000; # 10s
                  Retries = 3;
                  StartPeriod = 5000000000; # 5s
                };
              };
            };

          scanner-nsfw-oci = pkgs.dockerTools.streamLayeredImage {
            name = "scanner-nsfw";
            tag = "latest";
            contents = [
              pkgs.cacert
              python-env
            ];
            config = {
              WorkingDir = "/scanner-nsfw";
              Entrypoint = [
                "${pkgs.tini}/bin/tini"
                "--"
                "${python-env}/bin/uvicorn"
              ];
              Cmd = [ "app:app" "--host" "0.0.0.0" "--port" "4100" ];
            };
          };
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          # checks = self.checks.${system};
        };
      });
}
