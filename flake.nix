{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk/master";
  };

  outputs =
    {
      nixpkgs,
      utils,
      naersk,
      ...
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk' = pkgs.callPackage naersk { };
        deps = with pkgs; [
          pkg-config
          udev
          alsa-lib
          vulkan-loader
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          libxkbcommon
          wayland
        ];
      in
      {
        packages.default = naersk'.buildPackage {
          src = ./.;
          buildInputs = deps;
          # xkbcommon use dlopen to load, so we need to add this to runtime dependencies manualy
          runtimeDependencies = [ "libxkbcommon.so.0" ];
        };

        devShell =
          with pkgs;
          mkShell rec {
            buildInputs = [
              cargo
              rustc
              rustfmt
              clippy
              rust-analyzer
            ] ++ deps;
            # xkbcommon use dlopen to load, so we need this envvar in dev shell
            LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
          };
      }
    );
}
