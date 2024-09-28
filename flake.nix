{
  description = "Monitor the Freedesktop Notifications D-Bus interface and copy 2FA codes to clipboard";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = crane.mkLib pkgs;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          buildInputs = [ ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
        };

        notifications-dbus-mon = craneLib.buildPackage (
          commonArgs // { cargoArtifacts = craneLib.buildDepsOnly commonArgs; }
        );
      in
      {
        checks = {
          inherit notifications-dbus-mon;
        };

        packages.default = notifications-dbus-mon;

        apps.default = flake-utils.lib.mkApp { drv = notifications-dbus-mon; };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            pkgs.rust-analyzer
            pkgs.zbus-xmlgen
          ];
        };
      }
    );
}
