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
    pkgs.alsa-lib
    pkgs.alsa-lib.dev
    pkgs.atk
    pkgs.atk.dev
    pkgs.gtk3
    pkgs.gtk3.dev
    pkgs.pango
    pkgs.pango.dev
    pkgs.cairo
    pkgs.cairo.dev
    pkgs.gdk-pixbuf
    pkgs.gdk-pixbuf.dev
    pkgs.webkitgtk_4_1
    pkgs.webkitgtk_4_1.dev
    pkgs.libsoup_3
    pkgs.libsoup_3.dev
    pkgs.librsvg
    pkgs.librsvg.dev
    pkgs.dbus
    pkgs.dbus.dev
    pkgs.llvmPackages.libclang
    pkgs.libv4l
    pkgs.libv4l.dev
    pkgs.libxkbcommon
  ];

  env = {
    LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.linuxHeaders}/include";
    LD_LIBRARY_PATH = "${pkgs.libxkbcommon}/lib";
  };

  enterShell = ''
    export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
    export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.linuxHeaders}/include"
    echo ""
    echo "Rust toolchain: $(rustc --version)"
    echo ""
  '';

  # Services in ./services
  processes = lib.mapAttrs' (name: _:
    lib.nameValuePair name {
      exec = "cargo run -p ${name}";
    }
  ) (lib.filterAttrs (_name: type: type == "directory") (builtins.readDir ./services))
  // {
    # Settings app (hooks up the Freya devtools server on 127.0.0.1:7354)
    settings = {
      exec = "cargo run -p settings";
    };
    # Freya DevTools companion app; connects to the settings app's inspector server.
    # `settings` must be running for devtools to connect.
    devtools = {
      exec = "./.devenv/state/cargo-install/bin/freya-devtools-app";
    };
  };

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