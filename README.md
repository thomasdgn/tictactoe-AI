# Ultimate Tic Tac Toe AI

Ce projet contient un moteur d'Ultimate Tic Tac Toe ecrit en Rust stable, sans dependance externe, pense pour un challenge universitaire en environnement terminal simple comme Google Colab.

L'objectif n'est pas de resoudre parfaitement le jeu, mais de produire une IA pratique, rapide, robuste, et difficile a battre sous contrainte de temps.

## Etat du projet

Le programme compile et tourne en release avec :

```bash
cargo check
cargo build --release
cargo run --release
```

Le moteur supporte :

- humain vs IA
- IA vs IA
- choix du joueur qui commence
- saisie humaine sous la forme `colonne ligne`, de `1 1` a `9 9`
- parties repetees sans relancer le programme
- mode d'entrainement/tuning par self-play avec `--train`
- mode benchmark exportant des resultats en CSV avec `--bench`
- mode tournoi de profils heuristiques avec `--tournament`
- tests unitaires des regles principales avec `cargo test`

Le code principal est dans `src/main.rs`.

## Architecture

Le projet reste volontairement dans un seul fichier Rust pour faciliter le rendu, la compilation et l'execution en Colab. Le fichier est cependant separe en blocs logiques :

1. constantes et poids d'evaluation
2. representation du plateau
3. detection des victoires locales
4. detection des victoires globales
5. generation des coups legaux
6. application et annulation des coups
7. evaluation heuristique
8. moteur de recherche Minimax/Negamax
9. gestion du temps
10. interface texte
11. mode self-play et tuning
12. tests unitaires des regles

Cette organisation permet de garder un programme autonome tout en separant clairement les responsabilites.

## Representation du plateau

Le plateau global 9x9 est stocke dans un tableau fixe de `81` cases.

Chaque case contient :

- `0` pour vide
- `1` pour X
- `2` pour O

Le moteur maintient aussi :

- `local_status[9]` pour savoir si chaque sous-plateau est ouvert, gagne par X, gagne par O, ou nul
- `global_status` pour le resultat global
- `current_player` pour le joueur courant
- `next_board` pour le sous-plateau impose au prochain coup
- `ply` pour le nombre de demi-coups joues
- `hash` pour la table de transposition

La representation est compacte, cache-friendly, et evite les allocations inutiles dans les fonctions appelees par la recherche.

## Regles implementees

Les regles d'Ultimate Tic Tac Toe sont gerees sans simplification :

- un coup est joue sur une coordonnee globale 9x9
- le coup envoie l'adversaire dans le sous-plateau correspondant a la position locale jouee
- si le sous-plateau de destination est deja gagne ou nul, le prochain joueur peut jouer dans n'importe quel sous-plateau encore ouvert
- un sous-plateau local est gagne par alignement de 3
- le plateau global est gagne par alignement de 3 sous-plateaux gagnes
- si tous les sous-plateaux sont fermes sans gagnant global, la partie est nulle

Les coups humains invalides sont refuses proprement.

## Recherche IA

L'IA respecte les contraintes du sujet :

- Minimax est utilise sous forme Negamax
- Alpha-Beta pruning est implemente
- aucun opening book
- aucune table de coups pre-calculee
- les decisions sont calculees a la volee
- la table de transposition sert uniquement de cache de recherche

Le moteur utilise aussi :

- iterative deepening
- aspiration windows
- Zobrist hashing
- transposition table
- principal variation via le meilleur coup stocke en table
- killer moves
- history heuristic
- move ordering tactique
- extensions tactiques simples en bord de recherche
- arret propre quand le temps expire

Si une recherche est interrompue par le temps, le moteur conserve le meilleur coup issu de la derniere profondeur entierement terminee.

## Evaluation heuristique

L'evaluation retourne un score absolu favorable a X, puis le moteur le convertit selon le joueur au trait pour le Negamax.

Elle combine plusieurs niveaux.

### Evaluation globale

Le moteur valorise :

- victoire globale immediate avec un score massif
- controle du centre macro
- controle des coins macro
- controle des bords macro
- lignes macro avec 2 sous-plateaux gagnes et une case encore ouverte
- penalites pour menaces macro adverses

Apres entrainement local, les poids par defaut favorisent davantage le centre macro et reduisent un peu la valeur des coins macro.

### Evaluation locale

Pour chaque sous-plateau ouvert, le moteur regarde :

- centre local
- coins locaux
- bords locaux
- une piece dans une ligne encore ouverte
- deux pieces dans une ligne avec une case vide
- menace adverse locale a bloquer

Un sous-plateau gagne a une forte valeur, mais sa valeur depend aussi de sa position dans le plateau global.

### Destination forcee

Ultimate Tic Tac Toe est surtout tactique parce que chaque coup impose souvent le prochain sous-plateau adverse.

Le moteur evalue donc explicitement :

- si le sous-plateau envoye est favorable au joueur qui va jouer
- combien de coups restent disponibles dans ce sous-plateau
- si l'adversaire est envoye dans une position contrainte
- si le coup libere l'adversaire en l'envoyant vers un plateau ferme

Ce terme est volontairement modere : il aide l'ordre strategique sans dominer les victoires locales ou globales.

## Move ordering

Avant de rechercher les coups, le moteur les trie avec une heuristique rapide.

Priorite approximative :

1. meilleur coup de la table de transposition
2. killer moves
3. coups ayant deja provoque des coupes via history heuristic
4. victoire globale immediate
5. blocage de menace globale
6. victoire locale
7. blocage local
8. coup envoyant l'adversaire vers un plateau faible ou contraint
9. centre local
10. coins locaux

Un bon ordre des coups est essentiel : Alpha-Beta coupe beaucoup plus quand les bons coups sont essayes tot.

## Mode interactif

Lancer :

```bash
cargo run --release
```

Le programme demande :

- mode `h` pour humain vs IA ou `a` pour IA vs IA
- joueur qui commence, `x` ou `o`
- profondeur maximale
- temps par coup en millisecondes, ou `0` pour recherche sans limite de temps
- cote humain si le mode humain vs IA est choisi

Pour jouer un coup humain, entrer :

```text
colonne ligne
```

Exemple :

```text
5 5
```

Les colonnes et lignes vont de 1 a 9.

## Mode benchmark CSV

Le mode benchmark lance plusieurs parties IA vs IA silencieuses et exporte un fichier CSV :

```bash
cargo run --release -- --bench <games> <depth> <time_ms> <output.csv>
```

Exemple :

```bash
cargo run --release -- --bench 50 4 200 benchmark.csv
```

Colonnes exportees :

- numero de partie
- joueur qui commence
- gagnant
- nombre de coups
- profondeur
- temps par coup
- nombre de coups d'ouverture randomises
- duree de la partie en millisecondes
- nombre total de noeuds explores
- nombre de coups vraiment recherches par l'IA
- profondeur moyenne atteinte
- recherches terminees completement
- recherches interrompues par le temps
- score moyen retourne par la recherche
- noeuds explores par milliseconde

Ce mode sert a comparer des reglages avant/apres, mesurer la stabilite, et produire des traces exploitables dans Colab ou dans un tableur.

## Mode entrainement

Le projet inclut un tuner par self-play :

```bash
cargo run --release -- --train
```

Syntaxe complete :

```bash
cargo run --release -- --train <rounds> <games> <depth> <time_ms>
```

Exemple rapide :

```bash
cargo run --release -- --train 2 3 3 80
```

Exemple plus serieux :

```bash
cargo run --release -- --train 5 12 4 200
```

Signification :

- `rounds` : nombre de passes de tuning
- `games` : nombre de paires de parties par candidat
- `depth` : profondeur de recherche pendant le tuning
- `time_ms` : budget par coup pendant le tuning

Le tuner compare des variantes de poids contre le meilleur profil courant. Chaque candidat joue des parties comme X et comme O. Les ouvertures sont legerement randomisees pendant le tuning pour eviter de comparer toujours la meme partie deterministe.

La generation des candidats combine maintenant :

- variations coordonnee par coordonnee
- variations positives et negatives
- mutations aleatoires sur plusieurs poids a la fois
- ouvertures randomisees de longueur variable

Important : ce mode ne cree pas d'opening book. Il ne stocke pas de dictionnaire de positions. Il sert uniquement a trouver de meilleurs poids generaux pour l'evaluation.

## Mode tournoi de profils

Le mode tournoi compare automatiquement plusieurs profils de poids et exporte un classement CSV :

```bash
cargo run --release -- --tournament <profiles> <games_per_pair> <depth> <time_ms> <generations> <output.csv>
```

Exemple rapide :

```bash
cargo run --release -- --tournament 8 2 3 100 2 tournament.csv
```

Exemple plus serieux :

```bash
cargo run --release -- --tournament 20 4 4 200 5 tournament.csv
```

Le programme genere des variantes autour des poids actuels, puis chaque profil affronte les autres comme X et comme O. En mode multi-generation, il garde les meilleurs profils, mute leurs poids, puis relance un tournoi. Le CSV final contient :

- rang
- identifiant du profil
- points
- victoires
- nulles
- defaites
- nombre de parties
- nombre de generations utilisees
- tous les poids heuristiques du profil

Ce mode est plus proche d'une selection evolutive que d'une memorisation de parties. Il reste compatible avec les contraintes du challenge, car l'IA ne consulte pas une base de coups pendant une partie.

## Peut-on memoriser toutes les parties ?

Pour le morpion classique 3x3, c'est realisable. Le nombre de parties legales est petit, et l'on peut construire une table parfaite par retro-analyse ou Minimax complet. Avec cette approche, une IA peut toujours au moins faire nul, et gagner si l'adversaire se trompe.

Pour l'Ultimate Tic Tac Toe, ce n'est pas comparable. L'espace de positions est immense : le jeu contient 81 cases et une contrainte de destination qui cree beaucoup plus de branches que le morpion classique. Stocker toutes les parties ou toutes les positions utiles serait beaucoup trop volumineux pour ce projet et reviendrait aussi a construire une base de coups pre-calculee, ce qui est interdit par les consignes.

L'approche adaptee ici est donc :

- utiliser Minimax/Alpha-Beta pour calculer les coups a la volee
- utiliser la table de transposition comme cache temporaire de recherche
- tuner les poids heuristiques avec self-play
- classer les profils par tournoi CSV
- ne pas embarquer de dictionnaire de coups dans l'IA finale

## Resultat du tuning actuel

Un premier tuning local avait ete lance avec :

```bash
cargo run --release -- --train 3 5 3 80
```

Ensuite, un tournoi plus serieux a ete lance dans Colab avec :

```bash
cargo run --release -- --tournament 24 4 4 250 5 tournament.csv
```

Ce tournoi a pris environ 35 minutes et a produit le profil gagnant suivant, maintenant integre dans les constantes par defaut :

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

Pour un meilleur entrainement avant competition, il faut lancer plus de parties, idealement dans Colab ou sur une machine qui peut tourner plusieurs minutes.

## Strategie d'entrainement recommandee

Pour ameliorer vraiment le moteur :

1. lancer un tuning court pour verifier que tout marche
2. lancer un tuning moyen a profondeur 3 pour explorer vite beaucoup de variantes
3. lancer un tuning plus lent a profondeur 4 ou 5 pour confirmer
4. ne retenir que les poids qui gagnent aussi contre l'ancien profil
5. tester en IA vs IA avec le temps officiel du challenge

Commandes conseillees :

```bash
cargo run --release -- --train 3 8 3 100
cargo run --release -- --train 5 12 4 200
cargo run --release -- --train 4 16 4 500
cargo run --release -- --tournament 20 4 4 200 5 tournament.csv
cargo run --release -- --bench 100 4 200 benchmark.csv
```

Si le temps officiel par coup est court, il vaut mieux entrainer avec un temps proche de l'evaluation finale.

## Google Colab

Dans Colab, creer une cellule terminal/shell et installer Rust si besoin :

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

Puis lancer :

```bash
cargo build --release
cargo run --release
```

Pour entrainer dans Colab :

```bash
cargo run --release -- --train 5 12 4 200
cargo run --release -- --tournament 20 4 4 200 5 tournament.csv
```

Colab est adapte a ce projet parce que le programme est autonome, textuel, sans interface graphique et sans crate externe.

## Parametres faciles a tuner

Les constantes les plus importantes sont en haut de `src/main.rs`.

Les plus sensibles :

- `MACRO_CENTER_WEIGHT`
- `MACRO_CORNER_WEIGHT`
- `LOCAL_WIN_WEIGHT`
- `LOCAL_TWO_WEIGHT`
- `LOCAL_BLOCK_TWO_WEIGHT`
- `DESTINATION_WEIGHT`
- `MOBILITY_WEIGHT`

Les parametres de recherche importants :

- profondeur maximale choisie au lancement
- temps par coup
- taille de la table de transposition via `TT_BITS`
- fenetre d'aspiration
- bonus de move ordering dans `order_moves`

## Verification

Commandes utiles avant rendu :

```bash
cargo check
cargo test
cargo clippy -- -D warnings
cargo build --release
cargo run --release
```

Test rapide IA vs IA avec entrees pipees :

```bash
printf "a\nx\n2\n50\nn\n" | cargo run --release
```

Sous PowerShell :

```powershell
@('a','x','2','50','n') | cargo run --release
```

## Limites actuelles

Le moteur est deja complet et jouable. Les principales pistes demandees ont ete avancees : tests, benchmark CSV, tuning plus varie, extensions tactiques et move ordering plus fin.

Il reste surtout des ameliorations de confort ou de recherche plus avancee :

- ajouter davantage de tests sur des parties completes et positions piegeuses
- exporter aussi les statistiques de noeuds et profondeurs dans le benchmark CSV
- comparer automatiquement plusieurs profils de poids dans un mini-tournoi
- ajouter un mode de configuration par fichier si l'on veut eviter de recompiler les poids
- separer le fichier en modules si le projet grossit

Ces ameliorations ne sont pas necessaires pour executer le programme, mais elles aideraient a gagner en fiabilite et en force.
