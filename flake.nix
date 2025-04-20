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
        packages.default = pkgs.stdenv.mkDerivation rec {
            pname = "akasha";
            version = "0.0.1";

            src = pkgs.fetchurl {
              url = "https://github.com/kevinJ045/akasha-rust/releases/download/v0.0.1/v0.0.1-linux.tar.gz";
              sha256 = "0zjrr3w51plakpjsh82rrb3w17h5vlfxzmxaxmr9niwqw5yj176c";
            };

            sourceRoot = ".";

            buildInputs = libraries;
            nativeBuildInputs = [ pkgs.makeWrapper ];

            installPhase = ''
              mkdir -p $out/bin
              cp bin/genshin-viewer $out/bin/akasha
              chmod +x $out/bin/akasha

              wrapProgram $out/bin/akasha \
                --set LD_LIBRARY_PATH ${pkgs.lib.makeLibraryPath libraries}
            '';

            meta = {
              description = "Akasha.cv desktop app";
              platforms = pkgs.lib.platforms.linux;
            };
          };

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
          name = "akasha";
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
