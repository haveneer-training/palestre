# D-005 — L'ordre des vérifications dans une instruction

- **Identifiant :** D-005
- **Date :** 2026-10-27
- **Auteurs :** Chaimae Faris
- **Statut :** proposée
- **Commits concernés :** —

## 1. Contexte

Plusieurs fautes peuvent s'appliquer à une même instruction. La spec fixe une partie de l'ordre : B.2 impose
décodage, puis gaz, puis exécution, et le corpus public montre que `StackUnderflow` passe avant toute autre
vérification (cas 046b, 047b, 054b, 063) et que `AlreadyActed` est levée sans rappel (cas 032). Pour le reste, la spec
est muette : `CONST k` hors de la table sur une pile pleine ? `CALL` avec une pile d'appels pleine et une cible
invalide ? `SENSE` sur une pile pleine, rappel posé ou non ?

## 2. Options envisagées

### Option A — « tout vérifier, puis agir »
Arité, validité des opérandes, place sur la pile, puis seulement le rappel.

Compromis : aucun rappel n'est posé si la réponse ne pourra pas être empilée. Mais c'est un ordre que le texte de B.3
ne décrit nulle part.

### Option B — « dépiler, calculer, empiler »
Lecture littérale de la notation `a b -- r` : l'arité d'abord, puis les étapes propres à l'instruction dans l'ordre du
texte de B.3 (lire la table, poser le rappel, empiler le cadre puis sauter), et enfin l'empilement du résultat, qui
peut lever `StackOverflow`.

Compromis : une règle simple, la même pour toutes les instructions. Mais elle pose un rappel `SENSE` même quand sa
réponse ne pourra pas être empilée.

## 3. Ce que l'assistant IA a proposé

L'option B. Ses conséquences sont épinglées par des tests :
- `CONST` hors de la table sur une pile pleine donne `BadConst` (tâche T04, test `const_reads_the_table_before_pushing`) ;
- `CALL` sur une pile d'appels pleine avec une mauvaise cible donnera `CallStackOverflow` (T06) ;
- `SENSE` et `RAND` sur une pile pleine poseront le rappel, puis lèveront `StackOverflow` (T07).

**Aucun cas public ne confirme ce choix.** Il faut le confronter aux autres groupes à la séance d'interopérabilité (J4).

Ce que nous en avons vérifié : *à compléter.*

## 4. Ce que nous avons décidé, et pourquoi

*À rédiger.*

## 5. Ce qui nous ferait changer d'avis

Un cas caché, ou une majorité de serveurs à J4, qui ne pose pas le rappel sur une pile pleine. Il faudrait alors
réviser cette entrée et inverser le test de T07.

## Historique

- 2026-10-27 — proposée.
