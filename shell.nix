# Environnement de développement pour Bevy sur NixOS.
# Usage :  nix-shell        (entre dans le shell)
#          cargo run        (compile et lance le jeu)
# ou directement :  nix-shell --run "cargo run"
{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  # Outils nécessaires à la compilation (trouvent alsa/udev via pkg-config).
  # cargo-watch : relance automatiquement le jeu à chaque sauvegarde de fichier.
  nativeBuildInputs = [ pkgs.pkg-config pkgs.cargo-watch ];

  buildInputs = [
    pkgs.alsa-lib        # audio
    pkgs.udev            # périphériques (manettes, etc.)
    pkgs.vulkan-loader   # rendu graphique
    pkgs.libxkbcommon    # clavier
    pkgs.wayland         # affichage Wayland
    # Repli X11 / XWayland au cas où :
    pkgs.libx11
    pkgs.libxcursor
    pkgs.libxi
    pkgs.libxrandr
  ];

  # Bevy charge certaines bibliothèques dynamiquement à l'exécution :
  # il faut donc les exposer aussi dans LD_LIBRARY_PATH.
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
    pkgs.vulkan-loader
    pkgs.libxkbcommon
    pkgs.wayland
    pkgs.alsa-lib
    pkgs.udev
  ];
}
