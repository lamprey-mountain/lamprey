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
        sandboxModule = import ./nix/sandbox.nix { inherit nixpkgs microvm; };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        sandbox = sandboxModule { inherit system pkgs; id = "agent"; };
        agent-sandbox = sandbox.agent-sandbox;
        spawn-sandbox = sandbox.spawn-sandbox;

        craneLib = crane.mkLib pkgs;

        baseInternalDeps = [ "crate-common" "crate-hakari" "crate-macros" ];
        baseInternalDepsCrates = [ "lamprey-common" "lamprey-hakari" "lamprey-macros" ];

        # this filter could probably be made stricter
        includeFilter = path: type:
          (builtins.match ".*\\.sql" path != null) ||
          (builtins.match ".*/\\.sqlx(/.*)?" path != null) ||
          (builtins.match ".*\\.html" path != null) ||
          (builtins.match ".*\\.wit" path != null) ||
          (builtins.match ".*/package\\.json" path != null) ||
          (builtins.match ".*/jsr\\.json" path != null) ||
          (builtins.match ".*/docs(/.*)?" path != null);

        filterSrcFor = dirs: pkgs.lib.cleanSourceWith {
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions (
              [
                ./Cargo.toml
                ./Cargo.lock
                ./.cargo

                # include every Cargo.toml so workspace parsing passes
                (pkgs.lib.fileset.fileFilter (f: f.name == "Cargo.toml") ./.)
              ]
              ++ (map (dir: ./. + "/${dir}") dirs)
            );
          };

          filter = path: type:
            (includeFilter path type) || (craneLib.filterCargoSources path type);
        };

        src = ./.;
        common = {
          inherit src;
          strictDeps = true;
          doCheck = false;

          # generate dummy sources for omitted crates
          postUnpack = ''
            find $sourceRoot -name Cargo.toml -print0 | while IFS= read -r -d "" toml; do
              dir=$(dirname "$toml")
              if [ "$dir" != "$sourceRoot" ] && [ ! -d "$dir/src" ]; then
                mkdir -p "$dir/src"
                touch "$dir/src/lib.rs" "$dir/src/main.rs"
              fi
            done
          '';

          buildInputs = with pkgs; [
            openssl
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
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

        # reuse commonly used lamprey crates
        internalCratesArtifacts = craneLib.cargoBuild (common // {
          pname = "lamprey-internal-crates";
          cargoArtifacts = cargoArtifacts;
          src = filterSrcFor baseInternalDeps;
          cargoExtraArgs = pkgs.lib.concatMapStringsSep " " (dep: "-p ${dep}") baseInternalDepsCrates;
        });

        mkCrate = name: dirs:
          craneLib.buildPackage (common // {
            cargoArtifacts = internalCratesArtifacts;
            pname = name;
            cargoExtraArgs = "-p ${name}";
            src = filterSrcFor (dirs ++ baseInternalDeps);
            env = {
              VERGEN_GIT_SHA = self.rev or self.dirtyRev;
            };
          });

        backend = craneLib.buildPackage (common // {
            cargoArtifacts = internalCratesArtifacts;
            pname = "lamprey-backend";
            cargoExtraArgs = "-p lamprey-backend --features lamprey-backend/embed-frontend";
            src = filterSrcFor ([
              "crate-backend"
              "crate-backend-core"
              "crate-backend-data-postgres"
              "crate-markdown"
              "crate-unfurl"
              "crate-script"
            ] ++ baseInternalDeps);
            env = {
              VERGEN_GIT_SHA = self.rev or self.dirtyRev;
              FRONTEND_DIST = frontend;
            };
        });

        bridge = mkCrate "lamprey-bridge" [ "crate-bridge" "crate-sdk" ];
        voice = mkCrate "lamprey-voice" [ "crate-voice" ];
        media = mkCrate "lamprey-media" [ "crate-media" ];
        scanner-malware = mkCrate "scanner-malware" [ "scanner-malware" ];

        wasm-cargo-artifacts = craneLib.buildDepsOnly (common // {
          pname = "lamprey-markdown-wasm-deps";
          src = filterSrcFor (baseInternalDeps ++ [ "crate-markdown" ]);

          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          cargoExtraArgs = "-p lamprey-markdown --features wasm";
        });

        wasm-bindgen-version = let
          lock = builtins.fromTOML (builtins.readFile ./Cargo.lock);
          pkg = builtins.head (builtins.filter (p: p.name == "wasm-bindgen") lock.package);
        in pkg.version;

        wasm-bindgen-cli = pkgs.wasm-bindgen-cli.overrideAttrs (old: rec {
          version = wasm-bindgen-version;

          src = pkgs.fetchCrate {
            pname = "wasm-bindgen-cli";
            inherit version;
            hash = "sha256-vtDQXL8FSgdutqXG7/rBUWgrYCtzdmeVQQkWkjasvZU=";
          };

          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            inherit src;
            hash = "sha256-eKe7uwneUYxejSbG/1hKqg6bSmtL0KQ9ojlazeqTi88=";
          };
        });

        wasm-markdown = craneLib.buildPackage (common // {
          cargoArtifacts = wasm-cargo-artifacts;
          pname = "lamprey-markdown-wasm";

          src = filterSrcFor (baseInternalDeps ++ [ "crate-markdown" ]);

          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          cargoExtraArgs = "-p lamprey-markdown --features wasm";

          nativeBuildInputs = common.nativeBuildInputs ++ [ wasm-bindgen-cli ];

          postBuild = ''
            wasm-bindgen \
              target/wasm32-unknown-unknown/release/lamprey_markdown.wasm \
              --out-dir $TMPDIR/pkg \
              --target web
            cp crate-markdown/package.json $TMPDIR/pkg/package.json
            cp crate-markdown/jsr.json $TMPDIR/pkg/jsr.json
          '';

          installPhase = ''
            mkdir -p $out
            cp -r $TMPDIR/pkg/* $out/
          '';

          doCheck = false;
        });

        frontend = pkgs.stdenvNoCC.mkDerivation (finalAttrs: rec {
          name = "frontend";
          pname = name;
          src = ./.;
          version = "0.0.0";

          nativeBuildInputs = with pkgs; [ nodejs pnpm pnpmConfigHook git ];

          VITE_GIT_SHA = self.rev or self.dirtyRev or "unknown";
          VITE_GIT_DIRTY = if (self ? rev) then "false" else "true";
          TWEMOJI_SPRITESHEETS = "${twemoji-spritesheets}";

          pnpmDepsHash = "sha256-m/yOm63rt/CzPoEpGokB7zdE1KSgVsvXiOATyfa21rk=";
          pnpmDeps = pkgs.fetchPnpmDeps {
            inherit (finalAttrs) src pname version;
            fetcherVersion = 3;
            hash = pnpmDepsHash;
          };

          buildPhase = ''
            cat > tsconfig.paths.json <<EOF
            {
              "compilerOptions": {
                "paths": {
                  "@/*": ["frontend/src/*"],
                  "@twemoji-spritesheets": ["${twemoji-spritesheets}"],
                  "@twemoji-spritesheets/*": ["${twemoji-spritesheets}/*"]
                }
              }
            }
            EOF

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

        twemoji-spritesheets = pkgs.stdenv.mkDerivation rec {
          pname = "twemoji-spritesheets";
          version = "16.0.0";

          emojiJson = pkgs.fetchurl {
            url = "https://raw.githubusercontent.com/iamcal/emoji-data/v${version}/emoji.json";
            hash = "sha256-HWAuZb6Idyv4zDaM4WuFXXGe7duv4SjUcbgCA/SU0p8=";
          };

          emojiSheet = pkgs.fetchurl {
            url = "https://raw.githubusercontent.com/iamcal/emoji-data/v${version}/sheets-indexed-256/sheet_twitter_64_indexed_256.png";
            hash = "sha256-OCNZhGRxmtfLOnuhRTe3hSyKMj4Ys598pPqbHpjpzGg=";
          };

          nativeBuildInputs = with pkgs; [
            jq
            libwebp
            libavif
            imagemagick
          ];

          dontUnpack = true;

          buildPhase = ''
            mkdir -p $out

            # strip json
            jq -c '[.[] | {u: .unified, x: .sheet_x, y: .sheet_y, s: .short_name}]' "$emojiJson" > "$out/data.json"

            # optimize -> webp
            cwebp -q 75 -m 6 ${emojiSheet} -o $out/sheet.webp

            # optimize -> avif
            avifenc --jobs all --speed 6 ${emojiSheet} $out/sheet.avif

            # optimize -> png
            magick ${emojiSheet} -colors 256 -quality 90 $out/sheet.png
          '';
        };
      in {
        packages = rec {
          inherit backend bridge voice media frontend scanner-malware wasm-markdown twemoji-spritesheets agent-sandbox spawn-sandbox;

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
        };

        devShells.default = craneLib.devShell {
          inputsFrom = [ backend ];
          packages = with pkgs; [nodejs pnpm chromium wasm-pack];
          env = {
            PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
            PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH = "${pkgs.chromium}/bin/chromium";
            TWEMOJI_SPRITESHEETS = "${twemoji-spritesheets}";
          };
          shellHook = ''
            cat > tsconfig.paths.json <<EOF
            {
              "compilerOptions": {
                "paths": {
                  "@/*": ["frontend/src/*"],
                  "@twemoji-spritesheets": ["${twemoji-spritesheets}"],
                  "@twemoji-spritesheets/*": ["${twemoji-spritesheets}/*"]
                }
              }
            }
            EOF
          '';
        };
      });
}
