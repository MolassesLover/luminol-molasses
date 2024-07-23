{ lib, rustPlatform, callPackage, fetchCrate, fetchFromGitHub, stdenv, cmake
, clang, ninja, pkg-config, gtk3, git, libxcb, speechd, openssl, alsa-lib
, libxkbcommon, autoPatchelfHook, makeBinaryWrapper }:

let
  fenix = callPackage (fetchFromGitHub {
    owner = "nix-community";
    repo = "fenix";
    rev = "8df0c074eac46e1f90e9e25c65ddbc2241717bb1";
    hash = "sha256-9s6WJOHUo/ChtV+1Kysf/BA0SIqmfl9SjJtkpiyRUWg=";
  }) { };
in rustPlatform.buildRustPackage rec {
  pname = "luminol";
  version = "v0.0.1";

  nativeBuildInputs = [
    autoPatchelfHook
    fenix.complete.toolchain
    pkg-config
    git
    cmake
    clang
    makeBinaryWrapper
  ];

  buildInputs = [ gtk3 libxcb speechd libxkbcommon openssl alsa-lib ];

  cmakeFlags = [ "-G Ninja" ];
  RUSTFLAGS = [ "-C" "linker=clang" "-Z" "macro-backtrace" ];

  src = ../../.;

  cargoHash = lib.fakeHash;

  cargoLock = {
    lockFile = ../../Cargo.lock;
    outputHashes = {
      "flume-0.11.0" = "sha256-3GyRZyxvQxpbgXoptCcd9Rvb5xcRQlNVeRpal7mFEzA=";
    };
  };

  postInstall = ''
    wrapProgram $out/bin/luminol \
      --set LD_LIBRARY_PATH ${libxkbcommon}/lib/libxkbcommon-x11.so
  '';
}
