{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  buildInputs = [
    pkgs.hello
    pkgs.rustup
    pkgs.rust-analyzer
    pkgs.cargo-watch
    pkgs.sccache

    # keep this line if you use bash
  ];
}
