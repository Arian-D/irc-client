{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/d6c71932130818840fc8fe9509cf50be8c64634f.tar.gz")  {},
  fenix ? import (fetchTarball "https://github.com/nix-community/fenix/archive/b19d93fdf9761e6101f8cb5765d638bacebd9a1b.tar.gz") {}
 }:

let
      toolchain = fenix.combine [
        fenix.minimal.toolchain
        fenix.stable.rustc
        fenix.stable.cargo
        #fenix.targets.wasm32-unknown-unknown.latest.rustc  # Key addition
        #  # Key addition
        fenix.targets.wasm32-unknown-unknown.latest.rust-std
        fenix.targets.x86_64-unknown-linux-gnu.latest.rust-std 
        fenix.targets.x86_64-unknown-linux-musl.latest.rust-std     
        ];
  in

pkgs.mkShell {
  buildInputs = with pkgs; [
    # rustc
    # cargo
    rustfmt
    clippy
    rust-analyzer
    toolchain
    trunk
  ];

  RUST_BACKTRACE = 1;
}
