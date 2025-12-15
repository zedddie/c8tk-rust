{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs.buildPackages; [
    rust-analyzer
    SDL2
    gcc
    rustfmt
    cargo
    rustc
  ];
}
