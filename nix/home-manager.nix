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

    store = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Install Finick Apps store";
      };
      extraPackages = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        example = [ "firefox" "vlc" ];
        description = "Extra nixpkgs attrs managed alongside Finick Apps generated apps.nix";
      };
      flatpaks = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        example = [ "com.spotify.Client" ];
        description = "Flatpak app IDs tracked as desired state by Finick Apps";
      };
    };
  };

  config = lib.mkMerge [
    (lib.mkIf cfg.enable {
    home.packages = lib.optionals (cfg.settings.enable || cfg.files.enable || cfg.overlay.enable || cfg.topbar.enable || cfg.store.enable) [ cfg.package ]
      ++ map (attr: lib.getAttrFromPath (lib.splitString "." attr) pkgs) cfg.store.extraPackages;

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

    # Hyprlang binds for hyprlang configs. On Hyprland >=0.55 Lua configs the
    # legacy `exec` dispatcher is wrapped as `hl.dispatch(exec ...)` and fails
    # with "attempt to perform arithmetic on a nil value (global 'exec')".
    # The overlay heals this at startup via `hl.bind(..., hl.dsp.exec_cmd(...))`
    # (`ensure_finick_default_bindings`), and all `hyprctl dispatch exec`
    # call sites use the Lua-first `system::hypr_exec` helper, so the launcher
    # works on both config providers.
    wayland.windowManager.hyprland.settings.bind = [
      # Session-lock client first; Freya `locker` remains the
      # `finished`-event fallback resolved via FINICK_LOCKER_BIN/PATH.
      "SUPER, L, exec, ${cfg.package}/bin/finick-lock"
      "SUPER, SPACE, exec, ${cfg.package}/bin/launcher"
    ];

    # No window rules for the locker: ext-session-lock surfaces are not
    # windows, so float/pin/noanim for class `locker` are obsolete. Overlay,
    # panel, launcher and modal rules below are unaffected. (The Freya
    # fallback locker re-floats/pins itself at runtime when it has to show.)
    wayland.windowManager.hyprland.settings.windowrulev2 = [
      "float, class:^(launcher)$"
      "pin, class:^(launcher)$"
      "noanim, class:^(launcher)$"
      "noshadow, class:^(launcher)$"
      "noblur, class:^(launcher)$"
      "noborder, class:^(launcher)$"
      "rounding 20, class:^(launcher)$"
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
    })
    # /etc/pam.d/finick-lock packaging. `security` only exists in the NixOS
    # module context, not in standalone home-manager, so this branch is a
    # no-op there (optionalAttrs is lazy in the false case). Non-NixOS:
    # sudo install -m 644 nix/finick-lock.pam /etc/pam.d/finick-lock
    (lib.optionalAttrs (config ? security) {
      security.pam.services.finick-lock = lib.mkIf cfg.enable {
        text = builtins.readFile ./finick-lock.pam;
      };
    })
  ];
}
