# to build use nix build -f default.nix

with import <nixpkgs> {
  overlays = [
    (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
  ];
};
let
  rustPlatform = makeRustPlatform {
    cargo = rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
    rustc = rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
  };
  manifest = (pkgs.lib.importTOML ./crates/way-edges/Cargo.toml).package;
in

rustPlatform.buildRustPackage {
  pname = manifest.name;
  version = manifest.version;

  buildInputs = with pkgs; [
    libxkbcommon
    cairo
    libpulseaudio
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  cargoLock = {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  src = pkgs.lib.cleanSource ./.;

  RUSTFLAGS = [
    "--cfg tokio_unstable"
    "--cfg tokio_uring"
  ];
}
