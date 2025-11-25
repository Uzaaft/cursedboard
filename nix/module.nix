{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.cursedboard;
in {
  options.services.cursedboard = {
    enable = lib.mkEnableOption "cursedboard clipboard sync";

    package = lib.mkPackageOption pkgs "cursedboard" {};

    name = lib.mkOption {
      type = lib.types.str;
      default = config.networking.hostName;
      description = "Device name for mDNS discovery.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 42069;
      description = "TCP port for peer connections.";
    };

    pskFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "File containing the pre-shared key. If null, uses default PSK.";
    };

    pollMs = lib.mkOption {
      type = lib.types.int;
      default = 500;
      description = "Clipboard polling interval in milliseconds.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      description = "User to run cursedboard as. Required for clipboard access.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Open firewall port for cursedboard.";
    };
  };

  config = lib.mkIf cfg.enable {
    networking.firewall = lib.mkIf cfg.openFirewall {
      allowedTCPPorts = [cfg.port];
      allowedUDPPorts = [5353];
    };

    systemd.user.services.cursedboard = {
      description = "Cursedboard clipboard sync";
      wantedBy = ["graphical-session.target"];
      after = ["graphical-session.target"];

      serviceConfig = {
        ExecStart = let
          pskArg =
            if cfg.pskFile != null
            then "--psk $(cat ${cfg.pskFile})"
            else "";
        in ''
          ${cfg.package}/bin/cursedboard \
            --name ${lib.escapeShellArg cfg.name} \
            --port ${toString cfg.port} \
            --poll-ms ${toString cfg.pollMs} \
            ${pskArg}
        '';
        Restart = "on-failure";
        RestartSec = 5;
      };

      environment = {
        DISPLAY = ":0";
        WAYLAND_DISPLAY = "wayland-0";
      };
    };
  };
}
