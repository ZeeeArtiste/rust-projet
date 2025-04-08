# Simulation de Robots sur une Carte Générée par Bruit de Perlin 
## Antoine Granier Dany Derensy

Ce projet consiste en une simulation de robots évoluant sur une carte générée de manière procédurale à l'aide du bruit de Perlin. La simulation intègre plusieurs fonctionnalités dont :

- **Génération de terrain** : Création d'une carte avec obstacles et ressources (représentées par des caractères) grâce au bruit de Perlin.
- **Robots autonomes** : Deux types de robots sont implémentés :
  - **Explorer** : Se déplace aléatoirement et signale la présence de ressources.
  - **Miner** : Se déplace vers les ressources signalées pour les collecter et retourne à la base une fois son inventaire plein.
- **Interface utilisateur textuelle** : Visualisation en temps réel de la simulation à l'aide de [ratatui](https://github.com/ratatui-org/ratatui).
- **Simulation multi-threadée** : Chaque robot est géré sur un thread individuel, avec mise à jour parallèle de la carte et des logs.

## Prérequis

- **Rust**  
  Assurez-vous d'avoir installé [Rust et Cargo](https://www.rust-lang.org/fr/tools/install).

- **Dépendances Cargo** :
  - [`noise`](https://crates.io/crates/noise) pour la génération du bruit de Perlin.
  - [`rand`](https://crates.io/crates/rand) pour la génération de nombres aléatoires.
  - [`ratatui`](https://crates.io/crates/ratatui) pour l'interface textuelle.
  - [`ctrlc`](https://crates.io/crates/ctrlc) pour gérer l'arrêt propre via Ctrl-C.

## Installation

1. Clonez le dépôt dans votre répertoire local

2. Installez les dépendances et construisez le projet avec Cargo (cargo build, run)

## Principe de fonctionnement

La carte affichée comporte des obstacles (#), des ressources (M ou E) et la base (S).

Les robots sont représentés par :

X pour l'explorer.

R pour le miner.

X trouve les ressources et les communique à un miner qui vient la récolter, une fois l'inventaire plein il va se vider à la base.
Le deuxième miner reste à l'arrêt si on a pas besoin de lui (- de 1 ressource à collecter).

