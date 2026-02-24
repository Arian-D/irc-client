{ pkgs,
  fenix,
  mkNixPak,
  ...
 }:

  let
      toolchain = fenix.combine [
        fenix.minimal.toolchain
        fenix.stable.rustc
        fenix.stable.cargo
        fenix.targets.wasm32-unknown-unknown.latest.rust-std
        fenix.targets.x86_64-unknown-linux-gnu.latest.rust-std 
        fenix.targets.x86_64-unknown-linux-musl.latest.rust-std     
        ];

      sandboxed-env = pkgs.mkShell {
        buildInputs = with pkgs; [
        rustfmt
        clippy
        rust-analyzer
        toolchain
        trunk
        amp-cli
          ];
      };


      amp-sandboxed = mkNixPak {
        config = { sloth, ...} : {
          app.package = sandboxed-env;
          app.binPath = "bin/bash";
          dbus.enable = true;
          bubblewrap = {
            network = true;
            bind.rw = [
              (sloth.env "PWD")
              [
                (sloth.concat' sloth.homeDir "/.local/share/amp")
                (sloth.concat' sloth.homeDir "/.config/amp")
              ]
            ];

            bind.ro = [
              "/nix"
            ];
          };
        };
      };
  in

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustfmt
    clippy
    rust-analyzer
    toolchain
    trunk

    #amp-sandboxed.config.script
  ];

  RUST_BACKTRACE = 1;
}
