flake: { config, lib, pkgs, ... }:

let
  inherit (lib) mkEnableOption mkOption types;

  inherit (flake.packages.${pkgs.stdenv.hostPlatform.system}) tiberius;

  cfg = config.services.tiberius;
in
{
  options = {
    services.tiberius = {
      enable = mkEnableOption ''
        Enable Tiberius Image Board
      '';

      package = mkOption {
        type = types.package;
        default = flake.packages.${pkgs.stdenv.hostPlatform.system}.default;
        description = ''
          Tiberius Package to use
        '';
      };
    };
  };

  config = lib.mkIf cfg.enable {

    systemd.services.tiberius = {
      description = "Tiberius Image Board";

      after = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Restart = "on-failure";
        ExecStart = builtins.concatStringsSep " " [
          "${lib.getBin cfg.package}/bin/tiberus"
          "--todo"
        ];
        StateDirectory = "tiberius";
        StateDirectoryMode = "0750";

        CapabilityBoundingSet = [ "AF_NETLINK" "AF_INET" "AF_INET6" ];
        LockPersonality = true;
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        PrivateUsers = true;
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHome = true;
        ProtectHostname = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectSystem = "strict";
        ReadOnlyPaths = [ "/" ];
        RemoveIPC = true;
        RestrictAddressFamilies = [ "AF_NETLINK" "AF_INET" "AF_INET6" ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [ "@system-service" "~@privileged" "~@resources" "@pkey" ];
        UMask = "0027";
      };
    };
  };
}