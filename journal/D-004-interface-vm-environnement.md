# D-004 — Interface VM ↔ environnement : une machine à reprises

- **Identifiant :** D-004
- **Date :** 2026-10-27
- **Auteurs :** Chaimae Faris
- **Statut :** proposée
- **Commits concernés :** —

## 1. Contexte

`SENSE`, `ACT` et `RAND` interrogent un environnement : l'arène en local, le client réseau sous l'annexe K (K.1, K.6).
Le sujet interdit `serde` dans la boucle interne de la VM et toute allocation pendant un tour (README §3). K.6 ajoute
qu'un rappel qui échoue (client parti) doit fermer la connexion **sans** `RESULT`.

## 2. Options envisagées

### Option A — un trait `Env` appelé depuis la boucle
`fn run(&mut self, env: &mut impl Env)`.

Compromis : simple et familier. Mais l'implémentation réseau sérialise du JSON et fait des E/S *à l'intérieur* de la
boucle, et un échec réseau impose un type d'erreur générique (`Env::Error`) qui traverse toute la VM.

### Option B — une machine à reprises
`run(self) -> Step` rend la main à chaque rappel, avec un jeton typé (`SenseCall`, `ActCall`, `RandCall`) dont la
méthode `answer` relance la machine.

Compromis : la boucle reste pure (ni E/S, ni JSON, ni allocation), et l'appelant (serveur, arène, corpus) gère ses
erreurs chez lui. Répondre à une question qui n'a pas été posée est impossible à écrire, car chaque `answer` consomme
son jeton. En contrepartie, la machine (environ 2,7 Ko) est déplacée par valeur à chaque rappel : c'est une copie
mémoire, pas une allocation.

## 3. Ce que l'assistant IA a proposé

L'option B, plus un trait `Env` et une fonction `run_with(machine, env)` pour les environnements qui répondent
toujours (arène, corpus, client) ; le serveur, lui, pilote `Step` directement. Clippy signalera `large_enum_variant`
sur `Step` : on l'autorise localement, avec un commentaire qui renvoie à cette entrée, plutôt que d'allouer une `Box`
à chaque rappel.

L'assistant a aussi écrit le code de la tâche T04 (`crates/vm`) et ses 18 tests. À ce stade, `Step` n'a qu'une
variante, `Done` : les rappels arrivent avec la tâche T07.

Ce que nous en avons vérifié : *à compléter.*

## 4. Ce que nous avons décidé, et pourquoi

*À rédiger.*

## 5. Ce qui nous ferait changer d'avis

Une mesure montrant que la copie de la machine à chaque rappel pèse (peu probable : un tour pose au plus 200 rappels
`RAND` au budget de 1000), ou une v2 où l'environnement devrait observer la machine pendant le rappel.

## Historique

- 2026-10-27 — proposée.
