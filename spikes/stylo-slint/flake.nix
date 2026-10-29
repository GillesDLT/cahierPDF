# cahierPDF — devShell des spikes (jalon J0).
#
# Fournit une toolchain Rust issue de nixpkgs (rustc, cargo, clippy, rustfmt)
# et les dépendances système nécessaires à Slint 1.18 (backend winit + rendu
# femtovg/software) :
#   - fontconfig, freetype : polices système ;
#   - libxkbcommon, wayland, libX11 & co. : saisie clavier/souris via winit
#     (Wayland natif — cible principale — ou X11/XWayland) ;
#   - libGL : contexte OpenGL du rendu femtovg.
#
# Retour d'expérience (conteneur Debian, backend X11) : winit charge
# libxkbcommon-x11.so par dlopen et panique au démarrage si elle est absente.
# D'où le LD_LIBRARY_PATH ci-dessous, qui rend les bibliothèques trouvables
# à l'exécution et pas seulement à l'édition de liens.
#
# Toolchain : la version de rustc dépend du canal nixpkgs, elle n'est pas
# épinglée (voie la plus simple : aucun input supplémentaire). Risque noté :
# il faut rustc >= 1.92 (rust-version de Slint 1.18.1, lui-même en édition
# 2024) et >= 1.85 (édition 2024 de ce spike). À vérifier au premier
# `nix develop` : `rustc --version` puis `cargo check`.
#
# À VÉRIFIER sur la machine cible (nix indisponible dans le conteneur de
# développement) : `nix develop` puis `cargo run`, et confirmer sur la doc
# Slint de la version figée dans Cargo.lock que la liste des dépendances
# système est complète.
{
  description = "cahierPDF — spike stylo-slint : toolchain Rust (nixpkgs) + dépendances système Slint";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          slintSystemDeps = with pkgs; [
            fontconfig
            freetype
            libGL
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ];
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustc
              cargo
              clippy
              rustfmt
              pkg-config
            ];

            buildInputs = slintSystemDeps;

            # winit/glutin chargent plusieurs bibliothèques par dlopen.
            # À vérifier : selon le canal nixpkgs, le support X11 de
            # libxkbcommon (libxkbcommon-x11.so) peut être dans un paquet
            # séparé ; il n'est utile qu'en session X11/XWayland.
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath slintSystemDeps;
          };
        });
    };
}