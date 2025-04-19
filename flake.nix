{
  description = "A basic rust devshell flake";
  # inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        openssl = pkgs.openssl;
        libraries = with pkgs; [
          mesa
          libGL
          libGLU
          cargo
          rustc
          rustup
          rustfmt
          libxkbcommon
          wayland
          xorg.libX11
          xwayland
          wayland
          openssl
          pkg-config
        ];
      in {
        devShells.default =
          pkgs.mkShell {
            buildInputs = libraries;

            shellHook = ''
              export PKG_CONFIG_PATH="${openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
            '';
          };
      });
}
