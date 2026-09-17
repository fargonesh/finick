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

    wayland.windowManager.hyprland.settings.windowrulev2 = [
      "float, class:^(overlay)$"
      "pin, class:^(overlay)$"
      "noanim, class:^(overlay)$"
      "noshadow, class:^(overlay)$"
      "noborder, class:^(overlay)$"
      "float, class:^(overlay-panel)$"
      "pin, class:^(overlay-panel)$"
      "rounding 20, class:^(overlay-panel)$"
      "noanim, class:^(overlay-panel)$"
      "noshadow, class:^(overlay-panel)$"
      "noborder, class:^(overlay-panel)$"
      "float, class:^(overlay-modal)$"
      "pin, class:^(overlay-modal)$"
      "noanim, class:^(overlay-modal)$"
      "noshadow, class:^(overlay-modal)$"
      "noborder, class:^(overlay-modal)$"
      "float, class:^(overlay-clipboard)$"
      "pin, class:^(overlay-clipboard)$"
      "rounding 20, class:^(overlay-clipboard)$"
      "noanim, class:^(overlay-clipboard)$"
      "noshadow, class:^(overlay-clipboard)$"
      "noborder, class:^(overlay-clipboard)$"
      "float, class:^(overlay-screenshot)$"
      "pin, class:^(overlay-screenshot)$"
      "rounding 20, class:^(overlay-screenshot)$"
      "noanim, class:^(overlay-screenshot)$"
      "noshadow, class:^(overlay-screenshot)$"
      "noborder, class:^(overlay-screenshot)$"
      "float, class:^(overlay-notifications)$"
      "pin, class:^(overlay-notifications)$"
      "rounding 20, class:^(overlay-notifications)$"
      "noanim, class:^(overlay-notifications)$"
      "noshadow, class:^(overlay-notifications)$"
      "noborder, class:^(overlay-notifications)$"
    ];
  };
}
