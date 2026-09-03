{ pkgs, lib, config, inputs, ... }: 

{

   packages = [
    pkgs.git
    pkgs.lld
    pkgs.mold
    pkgs.devenv
    pkgs.openssl
    pkgs.xdotool
    pkgs.stdenv.cc
    pkgs.binutils
    pkgs.sqlite
    pkgs.pkg-config
    pkgs.gtk4
    pkgs.gtk4.dev
    pkgs.libadwaita
    pkgs.libadwaita.dev
    pkgs.glib
    pkgs.glib.dev
];

  enterShell = ''
    echo ""
    echo "Rust toolchain: $(rustc --version)"
    echo ""
    fi
  '';


  languages.rust = {
    enable = true;
    channel = "nightly";
  };

  pre-commit.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
    cargo-check.enable = true;
  };
}