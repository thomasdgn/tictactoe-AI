# Ultimate Tic Tac Toe AI

## Présentation du projet

Ce projet contient une intelligence artificielle capable de jouer à l’**Ultimate Tic Tac Toe**, une variante plus complexe du morpion classique.

Le projet a été développé en **Rust stable**, sans dépendance externe, avec pour objectif de produire une IA :

- rapide ;
- robuste ;
- difficile à battre ;
- compatible avec un environnement simple comme un terminal ou Google Colab ;
- adaptée à un challenge universitaire où les coups sont chronométrés.

L’objectif n’est pas de résoudre parfaitement tout l’arbre du jeu, ce qui serait trop coûteux, mais de construire une IA capable de prendre de bonnes décisions en temps limité grâce à une recherche intelligente et une heuristique bien réglée.

Le moteur repose principalement sur :

- Minimax sous forme Negamax ;
- élagage Alpha-Beta ;
- heuristique maison ;
- iterative deepening ;
- table de transposition ;
- Zobrist hashing ;
- move ordering avancé ;
- modes de benchmark, d’entraînement et de tournoi.

---

## Objectif du challenge

L’Ultimate Tic Tac Toe est un jeu à deux joueurs joué sur une grille de `9 x 9`, composée de neuf petits morpions `3 x 3`.

Le but est de gagner trois petits morpions alignés sur le grand plateau.

À chaque coup, la case jouée dans un petit morpion détermine le petit plateau dans lequel l’adversaire devra jouer au tour suivant. Cette contrainte rend le jeu beaucoup plus tactique qu’un morpion classique.

Dans ce projet, l’IA doit respecter plusieurs contraintes importantes :

- les décisions doivent être calculées à la volée ;
- l’utilisation d’un dictionnaire de coups ou d’un opening book est interdite ;
- le moteur doit reposer sur Minimax ;
- l’élagage Alpha-Beta est utilisé pour accélérer la recherche ;
- une heuristique est nécessaire pour éviter d’explorer tout l’arbre du jeu ;
- le programme doit pouvoir tourner sur Google Colab ;
- l’interface texte suffit.

Nous avons donc cherché à construire une IA pratique, rapide et stable, plutôt qu’une IA théoriquement parfaite mais trop lente ou trop fragile.

---

## Pourquoi Rust ?

Rust a été choisi pour plusieurs raisons.

D’abord, Rust est un langage compilé très rapide, ce qui est intéressant pour un moteur de recherche adversarial comme Minimax. Plus le moteur est rapide, plus il peut explorer de positions dans le même temps.

Ensuite, Rust permet de contrôler finement la mémoire. Dans ce projet, cela permet d’éviter les copies inutiles du plateau et de limiter les allocations dans les parties critiques du programme.

Enfin, Rust reste compatible avec une exécution simple en terminal et peut être utilisé sur Google Colab après installation de la toolchain Rust.

Ce choix permet donc d’avoir un moteur performant, tout en restant suffisamment lisible pour un rendu universitaire.

---

## État actuel du projet

Le programme compile et tourne en release avec :

```bash
cargo check
cargo build --release
cargo run --release
```

Le moteur supporte actuellement :

- humain contre IA ;
- IA contre IA ;
- choix du joueur qui commence ;
- saisie humaine sous la forme `colonne ligne`, de `1 1` à `9 9` ;
- parties répétées sans relancer le programme ;
- mode d’entraînement par self-play avec `--train` ;
- mode benchmark exportant des résultats en CSV avec `--bench` ;
- mode tournoi de profils heuristiques avec `--tournament` ;
- tests unitaires des règles principales avec `cargo test`.

Le code principal est situé dans :

```text
src/main.rs
```

---

## Structure du projet

Le projet est volontairement gardé simple.

```text
TICTACTOE-AI/
├── src/
│   └── main.rs
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .gitignore
```

Le fichier `src/main.rs` contient tout le moteur. Ce choix a été fait pour faciliter :

- le rendu du projet ;
- la compilation sur une nouvelle machine ;
- l’exécution sur Google Colab ;
- la lecture globale du code.

Même si le projet tient dans un seul fichier, le code est organisé en blocs logiques :

1. constantes et poids d’évaluation ;
2. représentation du plateau ;
3. détection des victoires locales ;
4. détection des victoires globales ;
5. génération des coups légaux ;
6. application et annulation des coups ;
7. évaluation heuristique ;
8. moteur de recherche Minimax / Negamax ;
9. gestion du temps ;
10. interface texte ;
11. modes self-play, benchmark et tournoi ;
12. tests unitaires.

Cette organisation permet d’avoir un programme autonome, tout en gardant une séparation claire entre les responsabilités.

---

## Représentation du plateau

Le plateau global `9 x 9` est stocké dans un tableau fixe de `81` cases.

Chaque case contient :

```text
0 = case vide
1 = joueur X
2 = joueur O
```

Le moteur maintient également plusieurs informations importantes :

- `local_status[9]` : statut de chaque petit morpion ;
- `global_status` : statut global de la partie ;
- `current_player` : joueur au trait ;
- `next_board` : petit plateau imposé au prochain coup ;
- `ply` : nombre de demi-coups joués ;
- `hash` : hash Zobrist de la position courante.

Cette représentation est compacte, rapide et adaptée à la recherche en profondeur.

Elle permet aussi d’éviter des allocations inutiles dans les fonctions les plus appelées par l’IA.

---

## Règles implémentées

Les règles d’Ultimate Tic Tac Toe sont gérées sans simplification.

Le moteur prend en compte les règles suivantes :

- un coup est joué sur une coordonnée globale `9 x 9` ;
- le coup envoie l’adversaire dans le sous-plateau correspondant à la position locale jouée ;
- si le sous-plateau de destination est déjà gagné ou nul, le prochain joueur peut jouer dans n’importe quel sous-plateau encore ouvert ;
- un sous-plateau local est gagné par un alignement de trois symboles ;
- le plateau global est gagné par un alignement de trois sous-plateaux gagnés ;
- si tous les sous-plateaux sont fermés sans gagnant global, la partie est déclarée nulle.

Les coups humains invalides sont refusés proprement par le programme.

---

## Recherche IA

L’IA respecte les contraintes du sujet.

Elle utilise :

- Minimax sous forme Negamax ;
- Alpha-Beta pruning ;
- aucune table de coups pré-calculée ;
- aucun opening book ;
- aucune mémorisation de parties ;
- une table de transposition utilisée uniquement comme cache de recherche.

Le moteur utilise aussi plusieurs optimisations classiques des moteurs de jeu :

- iterative deepening ;
- aspiration windows ;
- Zobrist hashing ;
- transposition table ;
- principal variation via le meilleur coup stocké en table ;
- killer moves ;
- history heuristic ;
- move ordering tactique ;
- extensions tactiques simples en bord de recherche ;
- arrêt propre quand le temps expire.

Si une recherche est interrompue par la limite de temps, le moteur conserve le meilleur coup issu de la dernière profondeur entièrement terminée.

Cela évite que l’IA perde du temps dans une recherche incomplète sans pouvoir jouer de bon coup.

---

## Évaluation heuristique

L’évaluation retourne un score absolu favorable à X, puis le moteur le convertit selon le joueur au trait pour le Negamax.

L’heuristique est construite sur plusieurs niveaux.

---

### Évaluation globale

Le moteur valorise :

- une victoire globale immédiate avec un score massif ;
- le contrôle du centre du grand plateau ;
- le contrôle des coins du grand plateau ;
- le contrôle des bords du grand plateau ;
- les lignes globales avec deux sous-plateaux gagnés et une troisième case encore ouverte ;
- les pénalités liées aux menaces globales adverses.

L’idée est de ne pas seulement gagner des petits morpions, mais de gagner les bons petits morpions : ceux qui permettent de former une ligne sur le grand plateau.

---

### Évaluation locale

Pour chaque sous-plateau encore ouvert, le moteur analyse :

- le centre local ;
- les coins locaux ;
- les bords locaux ;
- les lignes contenant une pièce et des cases libres ;
- les lignes contenant deux pièces et une case libre ;
- les menaces adverses à bloquer.

Un sous-plateau gagné a une forte valeur, mais cette valeur dépend aussi de sa position dans le plateau global.

Par exemple, gagner le centre du grand plateau est généralement plus intéressant que gagner un bord isolé.

---

### Destination forcée

L’une des particularités les plus importantes de l’Ultimate Tic Tac Toe est la destination forcée.

À chaque coup, on envoie l’adversaire dans un sous-plateau précis. Cette mécanique est essentielle, car un bon coup n’est pas seulement un coup qui améliore notre position : c’est aussi un coup qui limite les possibilités adverses.

Le moteur évalue donc explicitement :

- si le sous-plateau envoyé est favorable au joueur qui va jouer ;
- combien de coups restent disponibles dans ce sous-plateau ;
- si l’adversaire est envoyé dans une position contrainte ;
- si le coup libère l’adversaire en l’envoyant vers un plateau déjà fermé.

Ce terme reste volontairement modéré pour ne pas dominer les critères plus importants comme les victoires locales ou globales.

---

## Move ordering

Avant de rechercher les coups, le moteur les trie avec une heuristique rapide.

L’ordre approximatif de priorité est le suivant :

1. meilleur coup connu dans la table de transposition ;
2. killer moves ;
3. coups ayant déjà provoqué des coupes via la history heuristic ;
4. victoire globale immédiate ;
5. blocage d’une menace globale ;
6. victoire locale ;
7. blocage local ;
8. coup envoyant l’adversaire vers un plateau faible ou contraint ;
9. centre local ;
10. coins locaux.

Le move ordering est une partie très importante du moteur.

Un bon ordre des coups permet à Alpha-Beta de couper beaucoup plus de branches. Cela revient à explorer moins de positions tout en conservant une bonne qualité de décision.

---

## Installation complète sur une nouvelle machine avec VS Code

Cette section explique comment installer et lancer le projet de zéro sur une machine qui ne possède pas encore forcément Rust, Cargo ou les outils nécessaires.

---

### 1. Installer Visual Studio Code

Télécharger et installer Visual Studio Code.

Ensuite, ouvrir VS Code et installer l’extension suivante :

```text
rust-analyzer
```

Cette extension permet d’avoir :

- l’autocomplétion Rust ;
- les erreurs en direct ;
- la navigation dans le code ;
- une meilleure expérience de développement.

---

### 2. Installer Rust et Cargo

Rust s’installe avec `rustup`.

#### Sur Linux ou macOS

Dans un terminal :

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Choisir l’installation par défaut :

```text
1) Proceed with standard installation
```

Fermer puis rouvrir le terminal.

Vérifier ensuite l’installation :

```bash
rustc --version
cargo --version
```

Si les deux commandes affichent une version, Rust est correctement installé.

---

#### Sur Windows

Sur Windows, il est recommandé d’installer Rust via `rustup-init.exe`.

Après installation, ouvrir un nouveau terminal PowerShell ou le terminal intégré de VS Code, puis vérifier :

```powershell
rustc --version
cargo --version
```

Si Rust demande l’installation des outils Microsoft C++ Build Tools, accepter l’installation, car ils peuvent être nécessaires pour compiler certains projets Rust sur Windows.

---

### 3. Récupérer le projet

Deux possibilités existent.

#### Option A : avec Git

Dans le terminal :

```bash
git clone <URL_DU_REPO>
cd TICTACTOE-AI
```

#### Option B : avec un fichier ZIP

1. Télécharger le projet sous forme de ZIP.
2. Extraire le dossier.
3. Ouvrir le dossier `TICTACTOE-AI` dans VS Code.

Attention : il faut ouvrir le dossier qui contient directement `Cargo.toml`.

---

### 4. Vérifier que le projet est bien reconnu

Dans le terminal intégré de VS Code, vérifier que l’on est bien dans le dossier du projet.

La commande suivante doit afficher les fichiers du projet :

```bash
ls
```

Sur Windows PowerShell :

```powershell
dir
```

On doit voir notamment :

```text
Cargo.toml
Cargo.lock
README.md
src
```

---

### 5. Vérifier le code

Avant de compiler en release, lancer :

```bash
cargo check
```

Cette commande vérifie que le projet est correct sans produire le binaire final optimisé.

Ensuite, lancer les tests :

```bash
cargo test
```

Si tout passe, le projet est prêt à être compilé.

---

### 6. Compiler le projet

Pour compiler en mode optimisé :

```bash
cargo build --release
```

Le binaire final est généré dans :

```text
target/release/
```

En général, il n’est pas nécessaire d’aller directement dans ce dossier, car on peut lancer le programme avec `cargo run`.

---

### 7. Lancer le programme

Pour lancer le programme en mode normal :

```bash
cargo run --release
```

Le programme demande ensuite :

- le mode de jeu ;
- le joueur qui commence ;
- la profondeur maximale ;
- le temps par coup ;
- le côté humain si le mode humain contre IA est choisi.

---

## Mode interactif

Lancer :

```bash
cargo run --release
```

Le programme demande :

```text
h = humain vs IA
a = IA vs IA
```

Puis il demande le joueur qui commence :

```text
x ou o
```

Il demande ensuite :

- la profondeur maximale ;
- le temps par coup en millisecondes ;
- le côté humain si besoin.

Pour jouer un coup humain, entrer :

```text
colonne ligne
```

Exemple :

```text
5 5
```

Les colonnes et lignes vont de `1` à `9`.

Le programme affiche ensuite la grille mise à jour après chaque coup.

---

## Exemple de partie rapide IA contre IA

Pour lancer une partie IA contre IA manuellement :

```bash
cargo run --release
```

Puis entrer :

```text
a
x
4
250
```

Cela lance une partie IA contre IA avec :

- X qui commence ;
- profondeur maximale 4 ;
- 250 ms par coup.

---

## Mode benchmark CSV

Le mode benchmark lance plusieurs parties IA contre IA silencieuses et exporte un fichier CSV.

Syntaxe :

```bash
cargo run --release -- --bench <games> <depth> <time_ms> <output.csv>
```

Exemple :

```bash
cargo run --release -- --bench 50 4 200 benchmark.csv
```

Exemple plus sérieux :

```bash
cargo run --release -- --bench 200 4 250 benchmark.csv
```

Le CSV exporte notamment :

- numéro de partie ;
- joueur qui commence ;
- gagnant ;
- nombre de coups ;
- profondeur ;
- temps par coup ;
- nombre de coups d’ouverture randomisés ;
- durée de la partie en millisecondes ;
- nombre total de nœuds explorés ;
- nombre de coups vraiment recherchés par l’IA ;
- profondeur moyenne atteinte ;
- recherches terminées complètement ;
- recherches interrompues par le temps ;
- score moyen retourné par la recherche ;
- nœuds explorés par milliseconde.

Ce mode sert à comparer les réglages avant et après modification des poids heuristiques.

---

## Mode entraînement

Le projet inclut un tuner par self-play.

Syntaxe simple :

```bash
cargo run --release -- --train
```

Syntaxe complète :

```bash
cargo run --release -- --train <rounds> <games> <depth> <time_ms>
```

Exemple rapide :

```bash
cargo run --release -- --train 2 3 3 80
```

Exemple plus sérieux :

```bash
cargo run --release -- --train 5 12 4 200
```

Signification des paramètres :

- `rounds` : nombre de passes de tuning ;
- `games` : nombre de paires de parties par candidat ;
- `depth` : profondeur de recherche pendant le tuning ;
- `time_ms` : budget par coup pendant le tuning.

Le tuner compare des variantes de poids contre le meilleur profil courant. Chaque candidat joue des parties comme X et comme O.

Les ouvertures sont légèrement randomisées pendant le tuning afin d’éviter de comparer toujours la même partie déterministe.

La génération des candidats combine :

- variations coordonnée par coordonnée ;
- variations positives et négatives ;
- mutations aléatoires sur plusieurs poids à la fois ;
- ouvertures randomisées de longueur variable.

Important : ce mode ne crée pas d’opening book. Il ne stocke pas de dictionnaire de positions. Il sert uniquement à trouver de meilleurs poids généraux pour l’évaluation.

---

## Mode tournoi de profils

Le mode tournoi compare automatiquement plusieurs profils de poids et exporte un classement CSV.

Syntaxe :

```bash
cargo run --release -- --tournament <profiles> <games_per_pair> <depth> <time_ms> <generations> <output.csv>
```

Exemple rapide :

```bash
cargo run --release -- --tournament 8 2 3 100 2 tournament.csv
```

Exemple plus sérieux :

```bash
cargo run --release -- --tournament 20 4 4 200 5 tournament.csv
```

Commande utilisée pour le tuning Colab principal :

```bash
cargo run --release -- --tournament 24 4 4 250 5 tournament.csv
```

Le programme génère des variantes autour des poids actuels, puis chaque profil affronte les autres comme X et comme O.

En mode multi-génération, il conserve les meilleurs profils, mute leurs poids, puis relance un tournoi.

Le CSV final contient :

- rang ;
- identifiant du profil ;
- points ;
- victoires ;
- nulles ;
- défaites ;
- nombre de parties ;
- nombre de générations utilisées ;
- tous les poids heuristiques du profil.

Ce mode est plus proche d’une sélection évolutive que d’une mémorisation de parties. Il reste compatible avec les contraintes du challenge, car l’IA ne consulte pas une base de coups pendant une partie.

---

## Peut-on mémoriser toutes les parties ?

Pour le morpion classique `3 x 3`, cela serait possible. L’espace de jeu est suffisamment petit pour construire une table parfaite.

Pour l’Ultimate Tic Tac Toe, ce n’est pas réaliste dans le cadre de ce projet.

Le jeu contient `81` cases, avec une contrainte de destination qui rend l’arbre de recherche très grand. Stocker toutes les parties ou toutes les positions utiles serait trop volumineux et reviendrait à créer une base de coups pré-calculée, ce qui est interdit par les consignes.

L’approche retenue est donc :

- utiliser Minimax / Alpha-Beta pour calculer les coups à la volée ;
- utiliser la table de transposition comme cache temporaire pendant la recherche ;
- tuner les poids heuristiques avec self-play ;
- classer les profils par tournoi CSV ;
- ne pas embarquer de dictionnaire de coups dans l’IA finale.

---

## Résultat du tuning actuel

Un premier tuning local avait été lancé avec :

```bash
cargo run --release -- --train 3 5 3 80
```

Ensuite, un tournoi plus sérieux a été lancé sur Google Colab avec :

```bash
cargo run --release -- --tournament 24 4 4 250 5 tournament.csv
```

Ce tournoi a pris environ 35 minutes.

Résultat du tournoi :

```text
Generation 1: best profile 1 points=288 wins=79 draws=51 losses=54 step=14%
Generation 2: best profile 20 points=299 wins=86 draws=41 losses=57 step=10%
Generation 3: best profile 0 points=276 wins=76 draws=48 losses=60 step=7%
Generation 4: best profile 23 points=294 wins=84 draws=42 losses=58 step=5%
Generation 5: best profile 23 points=302 wins=84 draws=50 losses=50 step=4%

Tournament complete:
profiles=24
games_per_pair=4
depth=4
time_ms=250
generations=5
elapsed=2156790 ms
csv=tournament.csv

Winner profile 23:
points=302
wins=84
draws=50
losses=50
```

Le meilleur profil trouvé a été intégré dans les constantes par défaut :

```text
MACRO_CENTER_WEIGHT = 1344
MACRO_CORNER_WEIGHT = 361
MACRO_EDGE_WEIGHT = 420
LOCAL_WIN_WEIGHT = 2816
LOCAL_CENTER_WEIGHT = 33
LOCAL_CORNER_WEIGHT = 30
LOCAL_EDGE_WEIGHT = 12
LOCAL_TWO_WEIGHT = 260
LOCAL_ONE_WEIGHT = 28
LOCAL_BLOCK_TWO_WEIGHT = 97
DESTINATION_WEIGHT = 52
MOBILITY_WEIGHT = 3
CLOSED_BOARD_PENALTY = 76
```

Ce profil est moins défensif que les premiers réglages et semble mieux équilibrer les gains locaux, les menaces globales et la mobilité.

---

## Résultat benchmark actuel

Après le tournoi, un benchmark a été lancé avec :

```bash
cargo run --release -- --bench 200 4 250 benchmark.csv
```

Résultat obtenu :

```text
Benchmark complete:
games=200
X wins=88
O wins=66
draws=46
elapsed=39441 ms
csv=benchmark.csv
```

Ces résultats servent surtout à mesurer la stabilité et la vitesse du moteur. Pour valider définitivement un profil, il est préférable de le comparer directement contre l’ancien profil dans un mode duel.

---

## Stratégie d’entraînement recommandée

Pour améliorer progressivement le moteur :

1. lancer un tuning court pour vérifier que tout fonctionne ;
2. lancer un tuning moyen à profondeur 3 pour explorer rapidement beaucoup de variantes ;
3. lancer un tuning plus lent à profondeur 4 ou 5 pour confirmer ;
4. ne retenir que les poids qui gagnent aussi contre l’ancien profil ;
5. tester en IA contre IA avec un temps proche de celui du challenge.

Commandes conseillées :

```bash
cargo run --release -- --train 3 8 3 100
cargo run --release -- --train 5 12 4 200
cargo run --release -- --train 4 16 4 500
cargo run --release -- --tournament 20 4 4 200 5 tournament.csv
cargo run --release -- --bench 100 4 200 benchmark.csv
```

Si le temps officiel par coup est court, il vaut mieux entraîner avec un temps proche de l’évaluation finale.

---

## Exécution sur Google Colab

Google Colab est utile pour lancer des tournois plus longs sans bloquer son ordinateur.

Le programme est adapté à Colab car il est :

- autonome ;
- textuel ;
- sans interface graphique ;
- sans crate externe.

---

### 1. Préparer le projet en ZIP

Avant d’envoyer le projet sur Colab, il est conseillé de ne pas inclure le dossier `target`.

Le fichier `.gitignore` devrait contenir :

```gitignore
/target
*.csv
```

Créer ensuite un fichier ZIP du projet.

---

### 2. Importer le projet dans Colab

Dans une cellule Colab :

```python
from google.colab import files
uploaded = files.upload()
```

Sélectionner le fichier :

```text
TICTACTOE-AI.zip
```

---

### 3. Dézipper le projet

Dans une cellule Colab :

```bash
!unzip TICTACTOE-AI.zip
```

Puis :

```python
%cd TICTACTOE-AI
```

---

### 4. Installer Rust dans Colab

Dans une cellule :

```bash
!curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

Puis ajouter Cargo au PATH :

```python
import os
os.environ["PATH"] += ":/root/.cargo/bin"
```

Vérifier l’installation :

```bash
!rustc --version
!cargo --version
```

---

### 5. Compiler dans Colab

```bash
!cargo build --release
```

---

### 6. Lancer un tournoi dans Colab

Exemple sérieux :

```bash
!cargo run --release -- --tournament 24 4 4 250 5 tournament.csv
```

---

### 7. Lancer un benchmark dans Colab

```bash
!cargo run --release -- --bench 200 4 250 benchmark.csv
```

---

### 8. Lire les résultats CSV dans Colab

```python
import pandas as pd

df = pd.read_csv("tournament.csv")
df.head(10)
```

---

### 9. Télécharger les résultats

```python
from google.colab import files

files.download("tournament.csv")
files.download("benchmark.csv")
```

---

## Paramètres faciles à tuner

Les constantes les plus importantes sont en haut de `src/main.rs`.

Les poids les plus sensibles sont :

- `MACRO_CENTER_WEIGHT`
- `MACRO_CORNER_WEIGHT`
- `LOCAL_WIN_WEIGHT`
- `LOCAL_TWO_WEIGHT`
- `LOCAL_BLOCK_TWO_WEIGHT`
- `DESTINATION_WEIGHT`
- `MOBILITY_WEIGHT`

Les paramètres de recherche importants sont :

- profondeur maximale choisie au lancement ;
- temps par coup ;
- taille de la table de transposition via `TT_BITS` ;
- fenêtre d’aspiration ;
- bonus de move ordering dans `order_moves`.

Ces paramètres permettent d’ajuster le comportement de l’IA selon le temps disponible le jour du challenge.

---

## Commandes utiles avant rendu

Avant de rendre le projet, lancer :

```bash
cargo check
cargo test
cargo build --release
cargo run --release
```

Si `clippy` est installé :

```bash
cargo clippy -- -D warnings
```

Test rapide IA contre IA avec entrées pipeées sous Linux ou macOS :

```bash
printf "a\nx\n2\n50\nn\n" | cargo run --release
```

Sous PowerShell :

```powershell
@('a','x','2','50','n') | cargo run --release
```

---

## Problèmes fréquents

### `cargo` n’est pas reconnu

Rust n’est pas installé ou le terminal n’a pas été redémarré.

Solution :

```bash
rustc --version
cargo --version
```

Si les commandes ne fonctionnent pas, réinstaller Rust puis rouvrir le terminal.

---

### Le projet ne compile pas dans VS Code

Vérifier que le terminal est ouvert dans le bon dossier.

Il faut être dans le dossier qui contient :

```text
Cargo.toml
```

---

### Le programme est lent

Toujours lancer les tests de performance en release :

```bash
cargo run --release
```

Éviter :

```bash
cargo run
```

car le mode debug est beaucoup plus lent.

---

### Colab ne trouve pas Cargo

Après installation de Rust dans Colab, il faut ajouter Cargo au PATH :

```python
import os
os.environ["PATH"] += ":/root/.cargo/bin"
```

---

## Limites actuelles

Le moteur est complet et jouable, mais certaines améliorations restent possibles.

Limites principales :

- l’heuristique reste perfectible ;
- le moteur n’est pas parallélisé ;
- il n’y a pas d’interface graphique ;
- la force dépend fortement du temps accordé par coup ;
- les poids ont été optimisés par tournoi, mais pourraient être encore mieux confirmés par des duels directs entre profils.

---

## Améliorations possibles

Les améliorations futures les plus intéressantes seraient :

- ajouter davantage de tests sur des parties complètes ;
- tester des positions tactiques piégeuses ;
- ajouter un mode duel automatique entre ancien et nouveau profil ;
- améliorer la gestion du temps selon l’avancement de la partie ;
- ajouter du multi-threading ;
- tester des variantes de Late Move Reduction ;
- séparer le code en plusieurs modules si le projet grossit ;
- ajouter un mode de configuration par fichier pour changer les poids sans recompiler.

Ces améliorations ne sont pas indispensables pour exécuter le programme, mais elles pourraient augmenter la fiabilité et la force du moteur.

---

## Conclusion

Ce projet met en place une IA complète et performante pour l’Ultimate Tic Tac Toe.

Le moteur respecte les contraintes du challenge :

- utilisation de Minimax ;
- élagage Alpha-Beta ;
- décisions calculées à la volée ;
- pas de dictionnaire de coups ;
- heuristique maison ;
- exécution possible sur terminal et Google Colab.

Le choix de Rust permet d’obtenir un moteur rapide, capable d’explorer efficacement un grand nombre de positions en temps limité.

Le projet combine donc une approche algorithmique classique avec des optimisations pratiques issues des moteurs de jeux, afin de produire une IA solide, stable et compétitive dans un cadre universitaire.