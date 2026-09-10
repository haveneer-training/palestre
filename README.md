# Projet : Palestre — La machine à agents déterministe 🏛️⚙️

![AnticPalestre.png](assets/AnticPalestre.png)

## 📝 Historique des changements

* `Thu Sep 04 2026`: Version initiale pour les étudiants de 5IW et 5IBC.

## 🧩 1. Contexte et Objectifs

Bienvenue dans le projet **Palestre**. Vous allez implémenter une **machine virtuelle déterministe**, le **format
binaire** qu'elle exécute, et un **exécuteur distant** permettant d'exécuter des programmes agents via le réseau.
À l'échelle réduite, c'est exactement ce qui fait tourner un *smart contract* ou une fonction *edge* : un bytecode,
une machine à pile, un compteur de gaz, un état, et un protocole d'exécution que toute implémentation correcte
doit reproduire à l'identique.

**La spécification est commune à tous les groupes.** C'est la contrainte centrale du projet : votre bytecode devra
tourner sur la machine d'un autre groupe, et votre serveur d'exécution devra produire exactement le même résultat,
les mêmes traces, les mêmes fautes et le même gaz consommé que ceux des autres groupes pour un programme donné.
Une divergence d'un seul bit ou d'une seule unité de gaz est ce que l'industrie appelle un *consensus bug*. Vous allez
en trouver — c'est le but.

Le sujet se compose de deux documents : le présent **`SUJET.md`** et la spécification normative **`SPEC-PBC-v1.md`** — en
cas de contradiction, c'est elle qui fait foi, lisez-la en entier avant d'écrire une ligne.

### Pourquoi ce sujet pour vos deux filières

| Ce que vous écrivez              | Ce que ça devient en IBC       | Ce que ça devient en IW                |
|----------------------------------|--------------------------------|----------------------------------------|
| Bytecode `PBC`                   | bytecode EVM, eBPF de Solana   | bytecode WebAssembly                   |
| Budget de gaz par tour           | `gas limit`, coût d'exécution  | quota CPU d'une fonction *edge*        |
| Empreinte et persistance d'état  | *state root*                   | idempotence et rejeu d'un pipeline     |
| Divergence entre implémentations | *consensus bug*, *chain split* | non-déterminisme d'un cache distribué  |
| Vérificateur de bytecode (bonus) | vérificateur EVM / eBPF        | validation d'un module WASM non fiable |

**Le pont avec WebAssembly est plus direct que la métaphore.** WASM est lui aussi une machine à pile ; son format
binaire commence par un nombre magique (`\0asm`) suivi d'une version et de sections typées, exactement comme l'en-tête
`PBC1` de l'annexe A ; son exécution est spécifiée pour être déterministe ; et les runtimes qui l'hébergent mesurent le
travail effectué par un compteur — le *fuel* de `wasmtime` est le cousin de votre budget de gaz.

Deux points valent d'être retenus pour la suite. D'abord, **tout module WASM est validé avant d'être exécuté** :
typage de la pile, cohérence des branchements, bornes mémoire. C'est ce que fait le bonus « vérificateur statique » du
§6, en miniature. Ensuite, **WASM n'a pas de saut vers une adresse arbitraire** : son flot de contrôle est structuré en
blocs et en branchements relatifs, précisément pour que cette validation reste faisable en une passe linéaire.
`PBC v1` fait le choix inverse et l'assume (annexe A.3) — vous découvrirez en implémentant le vérificateur pourquoi
WASM, l'EVM avec son `JUMPDEST` et eBPF avec ses boucles bornées ont tous reculé devant le saut libre.

Enfin, l'écosystème WASM se valide comme votre projet : une **suite de tests de conformité commune** que chaque runtime
doit rejouer. C'est la même mécanique que la journée d'interopérabilité, à une autre échelle.

### Les piliers du projet

* **Souveraineté technique** : tout est développé *from scratch*, sauf les bibliothèques listées au §3. Le chargeur, le
  décodeur, l'assembleur, le moteur d'exécution et le protocole réseau sont écrits à la main.
* **Déterminisme** : c'est la propriété que tout le reste sert. Elle se démontre, elle ne se déclare pas.
* **Conformité** : la spécification normative fait foi. Lors de la soutenance, des tests automatisés confrontent vos
  programmes et vos exécuteurs à des cas de tests publics et cachés.
* **Interopérabilité** : à la journée d'interopérabilité (jalon J4), tous les groupes se croisent en salle : chaque
  bytecode contre chaque machine, chaque client contre chaque serveur.
* **Idiomes Rust** : une machine virtuelle est un cas d'école pour les `enum`, le *pattern matching* exhaustif, les
  types d'erreur typés, et l'arène d'indices plutôt que les références croisées.
* **Traçabilité des décisions** : le journal (§4) fait partie intégrante du livrable et sera évalué en détail.

### Les six invariants

Ils sont la spécification. Vous devez les garantir et savoir les **démontrer** :

1. **Déterminisme** — même bytecode, même état initial, même budget, mêmes réponses d'environnement ⇒ même trace,
   même état final, **et même gaz consommé**. Sur n'importe quelle machine, n'importe quel système, n'importe quelle
   implémentation conforme.
2. **Terminaison** — toute exécution s'arrête. Un agent qui épuise son budget de gaz est interrompu (`OutOfGas`).
3. **Atomicité du tour** — une faute annule les effets du tour sur l'agent et son état (mémoire). Ne sont pas annulés :
   le gaz consommé, la trace, et l'action tentée. La spécification est précise
   là-dessus (Annexe E.2) ; lisez-la deux fois.
4. **Isolation** — un agent ne peut lire ni écrire hors de sa mémoire (indices `0..=255`) et des bornes autorisées.
   Aucun dépassement n'est un `panic`.
5. **Absence de `panic`** — quelle que soit l'entrée, y compris une suite d'octets aléatoire, tronquée ou adversariale,
   la machine rend une erreur typée. Jamais un `panic`, jamais un débordement de pile natif, jamais une boucle infinie.
   Cet invariant se teste par *fuzzing*, pas par relecture.
6. **Coût annoncé** — le gaz consommé est fonction de la seule suite d'instructions exécutées. Jamais du temps écoulé,
   jamais de la machine hôte, jamais de la mémoire disponible.

---

## 🔍 2. Travail à réaliser

Le socle obligatoire attendu de chaque groupe s'articule autour de **deux composants fondamentaux** :

1. **Le Format (`pbc`)** — chargeur robuste, décodage et validation du format binaire `.pbc` (magic `PBC1`, version `1`,
   table de constantes `i64`, code). Ingestion sécurisée : zéro `panic`, aucune allocation non bornée, détection
   immédiate des binaires tronqués ou invalides (Annexe A.2). L'architecture interne, le découpage en modules, les
   types et les API de manipulation du bytecode restent votre entière responsabilité d'ingénierie.

2. **L'Exécuteur distant (`exec`)** — moteur d'exécution PBC et service réseau implémentant le protocole d'exécution
   distante défini dans l'**Annexe K**. Ce composant gère l'exécution du bytecode, la pile, la mémoire persistante,
   le prélèvement rigoureux du gaz (Annexe B et D), et délègue au client distant les interactions avec l'extérieur
   (`SENSE`, `ACT`, `RAND`) par un système de rappels bidirectionnels.

### Spécification CLI minimale

Pour permettre l'évaluation automatisée, les tests d'interopérabilité et la confrontation entre projets, votre projet
**DOIT** obligatoirement exposer la sous-commande suivante :

```bash
palestre exec --listen <adresse>
```

Cette commande démarre le serveur d'exécution réseau sur l'adresse spécifiée (par exemple `127.0.0.1:9000`), prêt à
traiter les trames du protocole défini dans l'Annexe K (`OPEN`, `SUBMIT`, `EXEC`, rappels `SENSE`/`ACT`/`RAND`).

### Outils de développement suggérés

Pour faciliter vos tests et la mise au point de vos agents, vous comprendrez rapidement l'intérêt de disposer d'outils
d'assemblage et de désassemblage (ou plus tard d'un langage source plus élaboré). Il vous est fortement conseillé
de développer vos propres commandes d'outillage (par exemple au sein du même binaire `palestre`) :

```bash
palestre asm agent.pasm -o agent.pbc       # assemblage textuel
palestre disasm agent.pbc                  # désassemblage lisible
palestre run agent.pbc                     # exécution locale d'un agent pour test
```

La syntaxe d'un éventuel assembleur textuel est libre (mnémoniques, étiquettes, directives). Si vous le développez, une
propriété fondamentale et particulièrement robuste à vérifier par vos tests est l'aller-retour :
```rust
asm(disasm(p)) == p   // pour tout binaire valide p
```

### Ouvertures et extensions libres : l'Arène et les Matches

La spécification normative formalise en Annexe E le déroulement exact d'un tour (`E.2`), le comportement des capteurs
`SENSE` (`E.3`), des actions `ACT` (`E.4`) et les conditions d'arbitrage et de victoire (`E.5`).

Vous êtes libres de concevoir votre propre moteur de simulation d'arène locale, permettant d'opposer deux programmes
sur une grille avec ressources et murs. Vous pouvez également imaginer des visualiseurs (TUI ou graphiques), des tournois
automatisés ou des langages de haut niveau. Ces développements sont vivement encouragés et valorisés dans le cadre des
ouvertures et bonus (§6).

---

## 🛠 3. Contraintes Techniques

### Langage et Bibliothèques

* **Langage** : Rust, édition 2021 ou 2024.
* **Pas de `unsafe`** : `#![forbid(unsafe_code)]` sur l'ensemble de votre moteur d'exécution et de décodage. Sans exception.
* **Pas d'`async`** : ni `async`/`await`, ni `tokio`, ni `futures`. Ce n'est pas une préférence de style : un runtime
  asynchrone introduit un ordonnancement non maîtrisé, source de non-déterminisme. La machine est **monotâche par construction** :
  une boucle, un compteur ordinal, aucune concurrence au cœur de l'exécution.
* **Concurrence** : attendue pour le service réseau (Annexe K), et **`std` uniquement** — `thread::scope`, `mpsc`, `Mutex`.
  Chaque connexion TCP traite ses requêtes de manière déterministe.
* **Qualité exigée** :
  * Minimisation des `unwrap()`, `expect()` et `panic!()` (tolérés uniquement dans les tests unitaires). Dans le moteur
    d'exécution et le décodeur, ils sont **strictement interdits** : l'invariant 5 les exclut par construction.
  * **Aucune allocation** dans la boucle d'exécution d'un tour. Pile, mémoire et pile d'appels sont de taille fixe et bornée :
    dimensionnez vos tampons à l'avance.
* **Ce qui sort du cours sera interrogé en priorité.** Vous répondez de chaque ligne de code présente dans votre dépôt. Une
  construction que vous savez expliquer, motiver et modifier en direct vous rapporte des points ; une construction opaque
  ou adoptée aveuglément vous en coûtera.
* **Dépendances autorisées** (toute autre bibliothèque doit être validée préalablement) :
  * `sha2` (calculs de hachage SHA-256)
  * `thiserror` (types d'erreurs idiomatiques)
  * `clap` (parsing d'arguments en ligne de commande)
  * `serde`, `serde_json` (sérialisation des trames réseau et configurations, jamais dans la boucle interne de la VM)
  * `rand` (pour vos tests ou la génération de cartes)
  * `proptest` (tests de propriétés)
  * `criterion` (benchmarking et mesure de performance)
  * `arbitrary`, `cargo-fuzz` (fuzzing et validation de robustesse)
* **Interdits explicites** :
  * Machines virtuelles et interpréteurs tiers (`wasmi`, `wasmtime`, `rhai`, `revm`, `mlua`, etc.).
  * Générateurs de parseurs (`pest`, `nom`, `lalrpop`, `chumsky`). Si vous réalisez un assembleur ou compilateur, votre parseur s'écrit à la main.

### Recommandations de tests et bonnes pratiques

* Tests unitaires exhaustifs par module (décodage, cycle de gaz, instructions, fautes, trames réseau).
* **Tests de propriétés (recommandé)** : l'utilisation d'une bibliothèque comme `proptest` constitue une excellente pratique pour éprouver automatiquement les invariants fondamentaux :
  * Déterminisme absolu de l'exécution pour une mémoire et des entrées identiques.
  * Terminaison garantie et respect du budget de gaz sur bytecode arbitraire.
  * Atomicité du tour en cas de faute (restauration d'état conforme à E.2).
  * Si vous implémentez un assembleur/désassembleur : l'aller-retour `asm(disasm(p)) == p`.
* **Fuzzing (bonne pratique)** : fuzzer le chargeur binaire et le décodeur (`cargo-fuzz` / `arbitrary`) est une pratique vivement encouragée pour consolider l'invariant 5 (absence totale de `panic` sur des suites d'octets hostiles, aléatoires ou corrompues).

---

## 🤝 4. Modalités de réalisation

* **Groupes** : 3 ou 4 personnes. Charge estimée : 30h par étudiant.
* **Git** : commits nominatifs. L'historique Git doit impérativement refléter une contribution réelle et équilibrée de
  chaque participant.
* **Interface graphique ou TUI** : facultative, valorisée en bonus.

### Calendrier et jalons indicatifs

Les jalons ci-dessous constituent des repères de progression suggérés pour organiser votre travail :

| Jalon   | Attendu                                                                                              | Vérification                                                   |
|---------|------------------------------------------------------------------------------------------------------|----------------------------------------------------------------|
| **J1**  | Dépôt configuré, CI en place, chargeur PBC, validation du binaire et décodeur d'instructions         | Tests unitaires du chargeur et du décodeur validés             |
| **J2**  | Moteur d'exécution complet : pile, arithmétique, gaz, sauts, appels, fautes typées                   | Tests des instructions, du budget de gaz et des 16 fautes      |
| **J3**  | Exécuteur distant : serveur réseau TCP implémentant les trames de l'Annexe K                         | Sessions, exécutions et rappels `SENSE`/`ACT`/`RAND` conformes |
| **J4**  | **Journée d'interopérabilité (en salle)** : confrontation des clients et serveurs entre groupes      | Échanges croisés de bytecode et de serveurs `palestre exec`    |
| **J5**  | Consolidation des tests, bonnes pratiques (propriétés, fuzzing), documentation d'architecture        | Suite de tests consolidée, documentation finalisée             |
| **Gel** | Dernier commit pris en compte la veille de la soutenance                                             | `git log`                                                      |

### Le journal de décisions (ADR) 📓

L'assistance par intelligence artificielle est **autorisée sans restriction**. En contrepartie, vous devez impérativement
tenir un journal de décisions d'architecture (ADR) versionné dans votre dépôt :

* **Un fichier par décision** : dans le répertoire `journal/`, nommé par exemple `journal/D-001-structure-vm.md`.
* **En-tête** : identifiant, date, auteurs, statut (`proposée` / `adoptée` / `révisée` / `abandonnée`), commits concernés.
* **Cinq sections obligatoires** :
  1. **Contexte** — le problème ou la question d'architecture rencontrée ;
  2. **Options envisagées** — au moins deux alternatives crédibles, avec leurs compromis ;
  3. **Ce que l'assistant IA a proposé** — fidèlement retranscrit, y compris lorsque la suggestion était exacte ou erronée ;
  4. **Ce que nous avons décidé, et pourquoi** — les raisons techniques du choix retenu par l'équipe ;
  5. **Ce qui nous ferait changer d'avis** — le signal technique ou la métrique qui montrerait que le choix était sous-optimal.
* **Règle d'antériorité** : les décisions doivent être commitées au fil de l'eau, avant ou en même temps que le code associé.
  Un journal rédigé rétrospectivement en bloc dans les derniers jours ne sera pas crédible.
* **Attendu** : au moins **dix entrées**, dont **au moins trois** où la proposition de l'assistant a été adoptée initialement
  sans compréhension immédiate totale. L'honnêteté intellectuelle et le recul critique sont directement notés.

---

## ⚖️ 5. Évaluation et Intelligence Artificielle

L'IA est un assistant, pas un substitut à votre compréhension. Le seul filtre d'évaluation fiable est le code défendu
au clavier, sans réseau, devant le jury. C'est pourquoi **8 points sur 20** se jouent en direct lors de la soutenance.

### Barème d'évaluation (sur 20 points)

1. **Architecture et Design (4 pts)** :
   * Clarté et modularité de la conception, séparation nette entre le format PBC, le moteur de calcul et la couche réseau.
   * Modélisation robuste des états et des erreurs : les états invalides sont inreprésentables.
   * Utilisation pertinente des traits et de la généricité en Rust.

2. **Conformité et Interopérabilité (4 pts)** :
   * Conformité stricte à la spécification `SPEC-PBC-v1.md` : jeu d'instructions B.3, cycle de gaz B.2, liste fermée des 16 fautes B.5.
   * Interopérabilité réseau : conformité complète au protocole de l'Annexe K via la commande `palestre exec --listen <adresse>`.
   * Matrice croisée lors de la soutenance : test de vos programmes sur les exécuteurs des autres groupes et de la référence.
   * Respect absolu des 6 invariants fondamentaux.

3. **Qualité Rust (4 pts)** :
   * Code idiomatique, `clippy` sans avertissement (`-D warnings`), formatage `rustfmt`.
   * Robustesse du chargeur et du moteur d'exécution : gestion rigoureuse des erreurs, absence de crash (`panic`) sur entrées hostiles ou corrompues.
   * Richesse de la suite de tests et valorisation des bonnes pratiques de test (tests de propriétés `proptest`, fuzzing du chargeur).
   * Prérequis d'admission (0 pt si non satisfait) : compilation propre sans avertissement, zéro `unsafe`, zéro `async`.

4. **Défense Orale et Chapeau (8 pts)** :
   * **Navigation (individuel, 2 pts)** : capacité immédiate à naviguer dans votre code sans hésitation, à pointer l'application
     d'un invariant spécifique ou à justifier un choix de typage ou de cycle de vie.
   * **Modifications en direct / Chapeau (collectif, 4 pts)** : épreuve de *MOB programming* avec **un seul clavier tournant
     toutes les 3 minutes**. Une carte tirée au sort impose une modification en direct (ajout d'une nouvelle instruction, adaptation d'un coût
     de gaz, gestion d'un cas limite réseau). À l'issue du temps imparti, la suite de tests doit être au vert.
   * **Analyse critique et Journal ADR (collectif, 2 pts)** : soutenance de vos choix d'architecture, analyse des limites de votre
     système et défense des entrées du journal ADR.

### La journée d'interopérabilité (Jalon J4)

Organisée en séance collective en décembre, elle permet de confronter vos implémentations en direct : tests de vos binaires PBC
sur les machines des camarades, vérification des échanges réseau via `palestre exec`. Aucun point n'est attribué ce jour-là :
c'est une occasion privilégiée de détecter les divergences de consensus et d'enrichir votre journal de décisions.

---

## 🎁 6. Pistes de Bonus

Pour dépasser le cadre du socle obligatoire et valoriser votre projet :

* **Assembleur et désassembleur textuels** : outillage de développement complet (compilation d'une syntaxe textuelle libre vers `.pbc`,
  désassemblage lisible, test de propriété `asm(disasm(p)) == p`).
* **Arène complète et simulation de tournois** : implémentation intégrale de l'Annexe E (monde, grille, tours, arbitrage),
  permettant de simuler des matches multi-agents et d'organiser des compétitions de stratégie en local.
* **Vérificateur statique de bytecode** : analyse statique préalable du bytecode garantissant *avant toute exécution*
  qu'aucun saut n'atterrit hors des limites ou sur une cible invalide, et bornant la hauteur maximale de pile (analogue aux vérificateurs WASM/EVM).
* **Fuzzing différentiel** : test automatisé confrontant votre exécuteur à celui d'un autre groupe pour détecter les cas de divergences subtiles.
* **Langage source de haut niveau** : conception d'un compilateur pour un langage source dédié (avec variables, boucles, fonctions),
  compilant vers le bytecode PBC, écrit entièrement à la main sans générateur de parseur.
* **Optimisations de la VM** : mise en œuvre de techniques avancées de machine virtuelle (*threaded code dispatch*, super-instructions)
  avec benchmarks rigoureux sous `criterion`.
* **Rejeu visuel (TUI ou Web/WASM)** : interface visuelle animant le déroulement d'une partie ou l'état de la machine tour par tour
  (en consommant passivement les données d'exécution sans recalculer les règles).
* **Cible `no_std` et portage WebAssembly** : rendre votre moteur d'exécution compilable en environnement embarqué sans `std` (`alloc` seul),
  ou exécutable dans un navigateur web via WebAssembly.
* **Preuve de fraude et exécution répartie** : modèle de calcul vérifiable où le client exécute et fournit des preuves d'état que le serveur vérifie.

---
*Affûtez votre code et rendez-vous à la palestre !* 🏛️⚙️
