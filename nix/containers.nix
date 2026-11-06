{
  pkgs,
  backend,
  bridge,
  voice,
  scanner-malware,
  scanner-nsfw,
}:
{
  backend-oci = pkgs.dockerTools.streamLayeredImage {
    name = "backend";
    tag = "latest";
    contents = [
      pkgs.dockerTools.caCertificates
      pkgs.ffmpeg-headless
      pkgs.file
      pkgs.curl
    ];
    config = {
      Entrypoint =
        [ "${pkgs.tini}/bin/tini" "--" "${backend}/bin/lamprey" ];
      Healthcheck = {
        Test = [
          "CMD"
          "${pkgs.curl}/bin/curl"
          "-f"
          "http://localhost:4000/api/v1/health"
        ];
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
      fakeRootCommands = ''
        mkdir -p /tmp /etc
        chmod 1777 /tmp
        echo "clamav:x:100:100::/var/lib/clamav:/bin/false" >> /etc/passwd
        echo "clamav:x:100:" >> /etc/group
        mkdir -p /var/log/clamav /var/run/clamav /run/clamav
        chown clamav -R /var/log/clamav /var/run/clamav /run/clamav
      '';
      enableFakechroot = true;
      config = {
        WorkingDir = "/";
        Entrypoint = [
          "${pkgs.tini}/bin/tini"
          "--"
          "${scanner-malware}/bin/scanner-malware"
          "--config"
          "/etc/scanner-malware.toml"
        ];
        User = "clamav";
        Healthcheck = {
          Test = [ "CMD-SHELL" "curl -f http://localhost:4101/health || exit 1" ];
          Interval = 30000000000; # 30s
          Timeout = 10000000000; # 10s
          Retries = 3;
          StartPeriod = 5000000000; # 5s
        };
        Volumes = {
          "/var/run/clamav" = {};
        };
      };
    };

  scanner-nsfw-oci =
    let
      scannerNsfwConfig = pkgs.writeTextFile {
        name = "scanner-nsfw-config";
        destination = "/etc/scanner-nsfw.toml";
        text = ''
          rust_log = "info"
          listen = { address = "0.0.0.0", port = 4100 }
          models_path = "/models"
        '';
      };
    in
    pkgs.dockerTools.streamLayeredImage {
      name = "scanner-nsfw";
      tag = "latest";
      contents = [
        pkgs.cacert
        pkgs.dockerTools.caCertificates
      ];
      config = {
        WorkingDir = "/";
        Entrypoint = [
          "${pkgs.tini}/bin/tini"
          "--"
          "${scanner-nsfw}/bin/scanner-nsfw"
          "--config"
          "/etc/scanner-nsfw.toml"
        ];
        Healthcheck = {
          Test = [ "CMD-SHELL" "curl -f http://localhost:4100/health || exit 1" ];
          Interval = 30000000000; # 30s
          Timeout = 10000000000; # 10s
          Retries = 3;
          StartPeriod = 5000000000; # 5s
        };
        Volumes = {
          "/models" = {};
        };
      };
    };
}
