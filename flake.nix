{
  description = "A Nix-flake-based Rust development environment";

  inputs.nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";

  outputs = { self, nixpkgs }:
    let
      supportedSystems =
        [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSupportedSystem = f:
        nixpkgs.lib.genAttrs supportedSystems
        (system: f { pkgs = import nixpkgs { inherit system; config = { 
          android_sdk.accept_license = true;
          allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) [
             "android-sdk-cmdline-tools"
             "android-sdk-tools"
           ];
        }; }; });
      overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));
      flake-conf = (builtins.fromTOML (builtins.readFile ./flake-conf.toml));
    in {
      devShells = forEachSupportedSystem ({ pkgs }: {

        default = 
    let
    android-env = pkgs.androidenv.composeAndroidPackages {
          #cmdLineToolsVersion = "8.0";
          #toolsVersion = "26.1.1";
          #platformToolsVersion = "30.0.5";
          #buildToolsVersions = [ "30.0.3" ];
          includeEmulator = false;
          #emulatorVersion = "30.3.4";
          #platformVersions = [ "28" "29" "30" ];
          includeSources = false;
          includeSystemImages = false;
          systemImageTypes = [ "google_apis_playstore" ];
          abiVersions = [ "armeabi-v7a" "arm64-v8a" ];
          #cmakeVersions = [ "3.10.2" ];
          includeNDK = true;
          #ndkVersions = ["22.0.7026061"];
          useGoogleAPIs = false;
          useGoogleTVAddOns = false;
          includeExtras = [
            "extras;google;gcm"
          ];
        };
    in 
        pkgs.mkShell rec {

          buildInputs = with pkgs; [
            clang
            # Replace llvmPackages with llvmPackages_X, where X is the latest LLVM version (at the time of writing, 16)
            llvmPackages.bintools
            rustup
            cargo-ndk
            android-env.androidsdk
          ] ++ lib.optional flake-conf.android androidenv.androidPkgs.ndk-bundle;
          RUSTC_VERSION = overrides.toolchain.channel;

          inherit (pkgs.lib.optionalAttrs flake-conf.android {
            ANDROID_SDK_ROOT = "${android-env.androidsdk}/libexec/android-sdk";
            ANDROID_NDK_ROOT = "${ANDROID_SDK_ROOT}/ndk-bundle";
          }) ANDROID_NDK_ROOT ANDROID_SDK_ROOT;
          #GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${ANDROID_SDK_ROOT}/build-tools/${buildToolsVersion}/aapt2";
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
