{
  description = "A Nix-flake-based Rust development environment";

  inputs.nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";

  outputs = { self, nixpkgs }:
    let
      supportedSystems =
        [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSupportedSystem = f:
        nixpkgs.lib.genAttrs supportedSystems
        (system: f { pkgs = import nixpkgs { inherit system; }; });
      overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));
    in {
      devShells = forEachSupportedSystem ({ pkgs }: {

        default = pkgs.mkShell {
          buildInputs = with pkgs; [
            clang
            # Replace llvmPackages with llvmPackages_X, where X is the latest LLVM version (at the time of writing, 16)
            llvmPackages.bintools
            rustup
          ];
          RUSTC_VERSION = overrides.toolchain.channel;
          # https://github.com/rust-lang/rust-bindgen#environment-variables
          LIBCLANG_PATH =
            pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
          shellHook = ''
            export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
            export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
          '';
          # Add precompiled library to rustc search path
          RUSTFLAGS = (builtins.map (a: "-L ${a}/lib") [
            # add libraries here (e.g. pkgs.libvmi)
          ]);
          LD_LIBRARY_PATH = with nixpkgs;
            lib.makeLibraryPath [
              # load external libraries that you need in your rust project here
            ];

          # Add glibc, clang, glib, and other headers to bindgen search path
          BINDGEN_EXTRA_CLANG_ARGS =
            # Includes normal include path
            (builtins.map (a: ''-I"${a}/include"'') [
              # add dev libraries here (e.g. pkgs.libvmi.dev)
              pkgs.glibc.dev
            ])
            # Includes with special directory paths
            ++ [
              ''
                -I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
            ];
        };
      });
    };
}
