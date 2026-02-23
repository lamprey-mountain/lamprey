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
              etcGroup = pkgs.writeTextFile {
                name = "etc-group";
                destination = "/etc/group";
                text = "clamav:x:1000:";
              };

              etcPasswd = pkgs.writeTextFile {
                name = "etc-passwd";
                destination = "/etc/passwd";
                text = "clamav:x:1000:1000:ClamAV:/var/lib/clamav:/usr/sbin/nologin";
              };

              clamdConfig = pkgs.writeTextFile {
                name = "clamd.conf";
                destination = "/etc/clamav/clamd.conf";
                text = ''
                  Foreground yes
                  LocalSocket /run/clamav/clamd.sock
                  LocalSocketMode 666
                  DatabaseDirectory /var/lib/clamav
                  PidFile /run/clamav/clamd.pid
                  LogSyslog no
                  LogVerbose no
                '';
              };

              freshclamConfig = pkgs.writeTextFile {
                name = "freshclam.conf";
                destination = "/etc/clamav/freshclam.conf";
                text = ''
                  DatabaseDirectory /var/lib/clamav
                  UpdateLogFile /var/log/clamav/freshclam.log
                  LogVerbose no
                  LogSyslog no
                  DatabaseMirror database.clamav.net
                  Checks 12
                '';
              };

              scannerMalwareConfig = pkgs.writeTextFile {
                name = "scanner-malware-config";
                destination = "/etc/scanner-malware.toml";
                text = ''
                  rust_log = "info"
                  clamav_host = "/run/clamav/clamd.sock"
                  listen = { address = "0.0.0.0", port = 4101 }
                '';
              };

              entrypoint = pkgs.writeShellApplication {
                name = "scanner-malware-entrypoint";
                runtimeInputs = [ pkgs.clamav pkgs.coreutils ];
                text = ''
                  # Prepare runtime directories
                  mkdir -p /run/clamav /var/lib/clamav /var/log/clamav
                  chown -R clamav:clamav /var/lib/clamav /var/log/clamav /run/clamav

                  # Run freshclam once to ensure the DB is up to date, then keep it running in background
                  echo "Running freshclam to update virus definitions..."
                  freshclam --config-file=/etc/clamav/freshclam.conf || true

                  # Start freshclam daemon in background for periodic updates
                  freshclam --config-file=/etc/clamav/freshclam.conf --daemon &

                  # Start clamd in background
                  echo "Starting clamd..."
                  clamd --config-file=/etc/clamav/clamd.conf &
                  CLAMD_PID=$!

                  # Wait for clamd socket to become available
                  echo "Waiting for clamd socket..."
                  for i in $(seq 1 30); do
                    if [ -S /run/clamav/clamd.sock ]; then
                      echo "clamd is ready"
                      break
                    fi
                    if [ "$i" -eq 30 ]; then
                      echo "clamd did not start in time" >&2
                      exit 1
                    fi
                    sleep 1
                  done

                  # Start the scanner (foreground); if it dies, kill clamd too
                  trap 'kill $CLAMD_PID 2>/dev/null' EXIT
                  exec ${scanner-malware}/bin/scanner-malware --config /etc/scanner-malware.toml
                '';
              };
            in
            pkgs.dockerTools.streamLayeredImage {
              name = "scanner-malware";
              tag = "latest";
              contents = [
                pkgs.dockerTools.caCertificates
                pkgs.clamav
                etcGroup
                etcPasswd
                clamdConfig
                freshclamConfig
                scannerMalwareConfig
                entrypoint
              ];
              config = {
                WorkingDir = "/";
                Entrypoint = [
                  "${pkgs.tini}/bin/tini"
                  "--"
                  "${entrypoint}/bin/scanner-malware-entrypoint"
                ];
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
