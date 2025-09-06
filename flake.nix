{
  description = "Spyfall CLI development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
        };

        # Create shell script binaries for common commands
        spyfall = pkgs.writeShellScriptBin "spyfall" ''
          cargo run -- "$@"
        '';

        build = pkgs.writeShellScriptBin "build" ''
          cargo build "$@"
        '';

        test = pkgs.writeShellScriptBin "test" ''
          cargo test "$@"
        '';

        lint = pkgs.writeShellScriptBin "lint" ''
          cargo clippy "$@"
        '';

        format = pkgs.writeShellScriptBin "format" ''
          cargo fmt "$@"
        '';
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-edit
            rust-analyzer

            # Aliases for common commands
            spyfall
            build
            test
            lint
            format
          ];

          shellHook = ''
            echo "🦀 Rust development environment loaded"
            echo "Available commands:"
            echo "  build          - Build the project (cargo build)"
            echo "  test           - Run tests (cargo test)"
            echo "  lint           - Run linter (cargo clippy)"
            echo "  format         - Format code (cargo fmt)"
            echo ""
            echo "Spyfall CLI usage:"
            echo "  1. List locations: spyfall locations"
            echo "  2. Generate challenge: spyfall challenge 'hotel'"
            echo "  3. Respond to challenge: spyfall respond '<base64-challenge>' 'hotel'"
            echo "  4. Verify response: spyfall verify '<base64-challenge>' '<base64-response>' 'hotel'"
            echo "  5. Brute force (spy mode): spyfall brute '<base64-challenge>' '<base64-response>'"
            echo ""
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "spyfall";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
      });
}
