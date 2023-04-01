{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.hello
    pkgs.rustup
    pkgs.rust-analyzer

    # keep this line if you use bash
    pkgs.bashInteractive
  ];
}
