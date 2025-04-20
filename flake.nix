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
          rustup
          libxkbcommon
          wayland
          xorg.libX11
          xwayland
          wayland
          openssl
          pkg-config
          pkgsCross.mingwW64.stdenv.cc
          xorg.libXcursor
          xorg.libXrandr
          readline
          xorg.libXi
          ncurses
          glibc
        ];
      in {
        packages.x86_64-linux = with nixpkgs.pkgs; rec {
          akasha = stdenv.mkDerivation rec {
            pname = "akasha";
            version = "1.0.0";

            src = fetchFromGitHub {
              owner = "kevinj045";
              repo = "akasha-rust";
              rev = "v1.0.0"; # Tag that corresponds to the release
              sha256 = "0d197ae735ad26c6446265b0bc3168fde4f1fac5890e1f8d3b137b4b53fc22cd"; # You can get this from `nix-prefetch-url` or `nix-prefetch-git`
            };

            buildInputs = libraries;

            meta = with lib; {
              description = "Akasha.cv desktop app";
              platforms = platforms.linux;
            };
          };
        };

        devShells.default =
          pkgs.mkShell {
            buildInputs = libraries;

            shellHook = ''
              export PKG_CONFIG_PATH="${openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
              
              if [ ! -f "$HOME/.cargo/env" ]; then
                rustup-init -y
              fi
              source "$HOME/.cargo/env"

              # Set default toolchain if not already set
              rustup show | grep -q "stable" || rustup default stable

              # Ensure windows target is installed
              rustup target add x86_64-pc-windows-gnu

            '';
          };
      });
}
