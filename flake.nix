{
  description = "hacky experiments galore";

  inputs = {
    crane.url = "github:ipetkov/crane?ref=master";
    flake-utils.url = "github:numtide/flake-utils?ref=main";
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";
    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, crane, flake-utils, nixpkgs, microvm }:
    let
      mkAgentSandbox = { system, id }:
        nixpkgs.lib.nixosSystem {
          inherit system;
          modules = [
            microvm.nixosModules.microvm
            ({ pkgs, lib, ... }: {
              networking.hostName = "lamprey-sandbox-${id}";

              # disabling bloat for speed
              documentation.enable = false;
              boot.enableContainers = false;

              microvm = {
                hypervisor = "qemu";
                socket = "microvm.socket";

                mem = 1024;
                vcpu = 2;

                shares = [
                  {
                    proto = "virtiofs";
                    tag = "ro-store";
                    source = "/nix/store";
                    mountPoint = "/nix/.ro-store";
                    socket = "ro-store.sock";
                  }
                  {
                    proto = "virtiofs";
                    tag = "workspace";
                    source = "/var/empty"; # placeholder, overridden by virtiofsd socket
                    mountPoint = "/workspace";
                    socket = "workspace.sock";
                  }

                  # caching
                  { proto = "virtiofs"; tag = "sccache";       source = "/var/empty";  mountPoint = "/cache/sccache";    socket = "sccache.sock"; }
                  { proto = "virtiofs"; tag = "cargo-home";    source = "/var/empty";  mountPoint = "/cache/cargo-home"; socket = "cargo-home.sock"; }
                  { proto = "virtiofs"; tag = "pnpm-store";    source = "/var/empty";  mountPoint = "/cache/pnpm-store"; socket = "pnpm-store.sock"; }
                ];

                writableStoreOverlay = "/nix/.rw-store";

                interfaces = [
                  {
                    type = "user";
                    id = "vm-nic";
                    mac = "02:00:00:00:00:01";
                  }
                ];

                qemu = {
                  extraArgs = [
                    "-device" "virtio-balloon-pci"
                    "-device" "virtio-serial-pci"
                  ];
                };
              };

              environment.systemPackages = with pkgs; [
                # standard toolkit
                git curl jq fd ripgrep

                # debugging
                htop

                # frontend
                nodejs pnpm chromium

                # backend
                cargo rustc
              ];

              users.users.agent = {
                isNormalUser = true;
                # home = "/workspace";
                extraGroups = [ "wheel" ];
                shell = pkgs.bashInteractive;
              };

              boot.kernelModules = [ "virtio_balloon" ];

              # use empty passwords for convenience
              users.users.root.password = "";
              security.sudo.wheelNeedsPassword = false;

              # autologin as agent user
              services.getty.autologinUser = "agent";
              services.getty.helpLine = lib.mkForce "";

              # # FIXME: shutdown on logout
              # systemd.services."getty@tty1" = {
              #   overrideStrategy = "asDropin";
              #   serviceConfig.Restart = lib.mkForce "no";
              #   postStop = "${pkgs.systemd}/bin/systemctl poweroff --force";
              # };

              # networking setup
              networking.useDHCP = false;
              networking.useNetworkd = true;
              systemd.network.enable = true;
              systemd.network.networks."10-lan" = {
                matchConfig.Name = "en* eth*";
                networkConfig.DHCP = "ipv4";
                networkConfig.IPv6AcceptRA = true;
              };
              services.resolved.enable = true;

              # run setup script from the host
              systemd.services.sandbox-setup = {
                description = "Run agent-sandbox-setup.sh from host";
                after = [
                  "workspace.mount" "network-online.target"
                  "sccache.mount" "cargo-home.mount" "pnpm-store.mount"
                ];
                wantedBy = [ "multi-user.target" ];
                path = with pkgs; [ bash coreutils util-linux ];
                serviceConfig = {
                  Type = "oneshot";
                  User = "agent";
                  WorkingDirectory = "/workspace";
                  StandardOutput = "journal+console";
                  StandardError = "journal+console";
                };
                script = ''
                  #!/usr/bin/env bash
                  /workspace/agent-sandbox-healthcheck.sh

                  if [ -f /workspace/agent-sandbox-setup.sh ]; then
                    echo "--- Executing agent-sandbox-setup.sh ---"
                    chmod +x /workspace/agent-sandbox-setup.sh
                    /workspace/agent-sandbox-setup.sh
                  else
                    echo "--- No setup script found at /workspace/agent-sandbox-setup.sh ---"
                  fi
                '';
              };

              system.stateVersion = "23.11";
            })
            ./agent-sandbox.nix
          ];
        };
    in
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
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          nativeBuildInputs = with pkgs; [
            perl
            mold
            clang
            lld
          ];

          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.clang}/bin/clang";
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=${pkgs.mold}/bin/mold";
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

          pnpmDepsHash = "sha256-1oQTy6syo64yfOTzuLFas+t6Sjx3LXdruumzg6ygppE=";
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

        agent-sandbox = mkAgentSandbox {
          inherit system;
          id = "agent";
        };
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
                Test = [ "CMD-SHELL" "curl -f http://localhost:4000/api/v1/health || exit 1" ];
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
            contents = [ pkgs.dockerTools.caCertificates pkgs.ffmpeg-headless ];
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
                  database_directory = "/var/run/clamav"
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
                # FIXME: the scanner errors if this is enabled?
                # Volumes = {
                #   "/var/run/clamav" = {};
                # };
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

          spawn-sandbox = pkgs.writeShellApplication {
            name = "spawn-sandbox";
            runtimeInputs = with pkgs; [ virtiofsd cloud-hypervisor coreutils slirp4netns ];
            text = ''
              WORKSPACE_DIR=$(pwd)
              WORKDIR=$(mktemp -d "/tmp/sandbox-XXXXXX")

              # kill background virtiofsd processes on exit
              cleanup() {
                kill "$(jobs -p)" 2>/dev/null || true
                rm -rf "$WORKDIR"
              }
              trap cleanup EXIT

              # socket names are relative paths, so into cloud-hypervisor will look for them here
              cd "$WORKDIR"

              # setup cache directories
              SCCACHE_DIR=''${SCCACHE_DIR:-$HOME/.cache/sccache}
              CARGO_HOME=''${CARGO_HOME:-$HOME/.cargo}
              PNPM_STORE=''${PNPM_STORE:-$HOME/.local/share/pnpm}
              mkdir -p "$SCCACHE_DIR" "$CARGO_HOME" "$PNPM_STORE"

              # --sandbox none is required for non-root users
              virtiofsd --socket-path="ro-store.sock"    --shared-dir=/nix/store   --sandbox none &
              virtiofsd --socket-path="workspace.sock"   --shared-dir="$WORKSPACE_DIR" --sandbox none &
              virtiofsd --socket-path="sccache.sock"     --shared-dir="$SCCACHE_DIR"   --sandbox none &
              virtiofsd --socket-path="cargo-home.sock"  --shared-dir="$CARGO_HOME"    --sandbox none &
              virtiofsd --socket-path="pnpm-store.sock"  --shared-dir="$PNPM_STORE"    --sandbox none &

              # wait for sockets to be ready
              for sock in ro-store workspace sccache cargo-home pnpm-store; do
                while [ ! -S "$sock.sock" ]; do sleep 0.1; done
              done

              # set runtime dir so other default sockets (like the hypervisor api)
              # go here instead of trying to hit root-owned /var/lib
              export MICROVM_RUNTIME_DIR="$WORKDIR"

              echo "booting microvm..."
              ${agent-sandbox.config.microvm.declaredRunner}/bin/microvm-run
            '';
          };
        };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [nodejs pnpm chromium];
          env = {
            PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
            PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH = "${pkgs.chromium}/bin/chromium";
          };
        };
      });
}
