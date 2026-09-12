{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.programs.finick.settings;
in {
  options.programs.finick.settings = {
    enable = mkEnableOption "Finick System Settings App and Daemon";

    # System-level configurations managed by the settings app
    network = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Allow Finick settings app to manage NetworkManager.";
      };
    };

    hyprland = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable Hyprland integration (gaps, colors, inputs).";
      };
    };
    
    screenTime = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable the background daemon for Screen Time tracking.";
      };
    };
    
    # User-managed state file that the GUI edits
    stateFile = mkOption {
      type = types.path;
      default = "/etc/finick-settings.json";
      description = "Path to the mutable JSON state file managed by the settings app.";
    };
  };

  config = mkIf cfg.enable {
    # 1. Enable required system services if managing network/bluetooth
    networking.networkmanager.enable = mkIf cfg.network.enable true;
    hardware.bluetooth.enable = true;

    # 2. Setup the background service for Screen Time and Indexing
    systemd.services.finickd = mkIf cfg.screenTime.enable {
      description = "Finick Settings Daemon (Screen Time & Indexing)";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.finick-settings}/bin/finickd";
        Restart = "always";
        User = "root"; # Needed for system-level stats (optional, depends on implementation)
      };
    };

    # 3. Ensure users are part of networkmanager group
    # Note: Requires users to be defined elsewhere
  };
}
