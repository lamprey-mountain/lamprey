{ nixpkgs, microvm }:
{ system, id, pkgs }:
let
  agent-sandbox = nixpkgs.lib.nixosSystem {
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
          wants = [ "network-online.target" ];
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
      ../agent-sandbox.nix
    ];
  };
in
{
  agent-sandbox = agent-sandbox.config.system.build.toplevel;

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
}
