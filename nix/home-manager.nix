{ config, lib, pkgs, ... }:
let
  cfg = config.programs.finick;
in
{
  options.programs.finick = {
    enable = lib.mkEnableOption "Finick desktop environment";

    package = lib.mkOption {
      type = lib.types.package;
      defaultText = lib.literalExpression "inputs.finick.packages.\${pkgs.system}.default";
      description = "Finick package containing all binaries";
    };

    overlay = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Whether to run overlay as a user service";
      };
    };

    topbar = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether to run topbar as a user service (deprecated, use overlay)";
      };
    };

    settings = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Install Finick Settings app";
      };
    };

    files = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Install Finick Files";
      };
    };

    daemon = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Run finickd as user service";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = lib.optionals (cfg.settings.enable || cfg.files.enable || cfg.overlay.enable || cfg.topbar.enable) [ cfg.package ];

    xdg.enable = true;
    xdg.mimeApps.enable = lib.mkDefault true;
    xdg.mimeApps.defaultApplications = lib.mkMerge [
      (lib.mkIf cfg.files.enable {
        "inode/directory" = "finick-files.desktop";
      })
    ];

    systemd.user.services.finick-overlay = lib.mkIf (cfg.overlay.enable || cfg.topbar.enable) {
      Unit = {
        Description = "Finick Overlay";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/overlay";
        Restart = "on-failure";
        RestartSec = 2;
      };
      Install.WantedBy = [ "graphical-session.target" "hyprland-session.target" ];
    };

    systemd.user.services.finickd = lib.mkIf cfg.daemon.enable {
      Unit = {
        Description = "Finick Settings Daemon";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/finickd";
        Restart = "on-failure";
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    systemd.user.services.finick-index = lib.mkIf cfg.files.enable {
      Unit.Description = "Finick File Indexer";
      Service = {
        ExecStart = "${cfg.package}/bin/index";
        Restart = "on-failure";
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
