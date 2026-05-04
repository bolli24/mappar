{pkgs ? import <nixpkgs> {}}: let
  libPath = with pkgs;
    lib.makeLibraryPath [
      libGL
      libxkbcommon
      wayland
      vulkan-loader
    ];
in
  pkgs.mkShell {
    buildInputs = with pkgs.buildPackages; [
      dbus
      openssl
      pkg-config
    ];
    LD_LIBRARY_PATH = libPath;
  }
