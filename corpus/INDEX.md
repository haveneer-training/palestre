# Cas standalone publics

Ces cas éprouvent une machine **seule**, exécutée comme l'annexe K l'exécute : un programme, une mémoire initiale, un
budget, les réponses aux rappels. Chaque cas se rejoue contre tout `palestre exec --listen <hôte:port>`.

| Nature | Répertoire | Cas | Contenu |
|---|---|---:|---|
| `edge` | `edge/` | 54 | cas limites : chacun vise une clause qu'une implémentation pressée rate ; la plupart fautent exprès, pas tous |
| `nominal` | `nominal/` | 11 | usages ordinaires et valides, aucune faute |

Le format des fichiers d'un cas est décrit dans `FORMAT.md`, à la racine du corpus. Un nom de cas suit
`NNN-catégorie-intitulé` ; un `b` après le numéro marque le second programme d'un même sujet.

## `header`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `001-header-bad-magic` | edge | A.1 | le magique n'est pas `PBC1` | refus : `BadHeader` |
| `002-header-nonzero-flags` | edge | A.1 | l'octet `flags` n'est pas `0x00` | refus : `BadHeader` |
| `003-header-trailing-byte` | edge | A.1 | un octet de trop après le code | refus : `BadHeader` |

## `decode`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `004-decode-unknown-opcode` | edge | B.2 | un opcode hors de B.3 | 1 exécution |
| `005-decode-truncated-operand` | edge | B.2 | un opérande coupé par la fin du code | 1 exécution |
| `006-decode-jump-into-operand` | edge | A.3 | un saut au milieu d'une instruction | 1 exécution |
| `007-decode-pc-past-end` | edge | B.2 | le compteur ordinal sort du code | 1 exécution |

## `stack`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `008-stack-overflow` | edge | B.1 | une 65ᵉ valeur sur une pile de 64 | 1 exécution |
| `009-stack-underflow` | edge | B.1 | `DROP` sur pile vide | 1 exécution |
| `010-stack-over-on-one-value` | edge | B.3 | `OVER` avec une seule valeur sur la pile | 1 exécution |
| `046-stack-rot-turns-three-deep` | edge | B.3 | `ROT` vaut `a b c -- b c a` | 1 exécution |
| `046b-stack-rot-turns-three-deep` | edge | B.3 | `ROT` sur deux valeurs : `StackUnderflow` avant tout déplacement | 1 exécution |
| `047-stack-constx-reads-the-table-by-computation` | edge | B.3 | `CONSTX` dépile son index : une table se lit par calcul | 1 exécution, rappels scriptés |
| `047b-stack-constx-reads-the-table-by-computation` | edge | B.3 | `CONSTX` sur pile vide : `StackUnderflow`, jamais `BadConst` | 1 exécution |
| `048-stack-constx-outside-the-table` | edge | B.3 | un index de constante dépilé négatif est `BadConst` | 1 exécution |
| `048b-stack-constx-outside-the-table` | edge | B.3 | un index dépilé de 65536 est `BadConst`, pas `constants[0]` | 1 exécution |
| `201-stack-everyday-shuffles` | nominal | B.3 | `PUSHI16`, `DUP`, `OVER`, `SWAP`, `ROT` sur de petites valeurs | 1 exécution |
| `204-stack-memory-within-a-turn` | nominal | B.1 | une cellule nommée et un petit tableau, écrits puis relus (`STORE`/`LOAD`, `STOREX`/`LOADX`) | 2 exécutions |
| `206-stack-named-constants` | nominal | B.3 | constantes par index fixe (`CONST`) et calculé (`CONSTX`) | 1 exécution |

## `arith`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `011-arith-wrapping-add` | edge | 0 | une addition qui enveloppe `i64::MAX` | 1 exécution |
| `012-arith-min-divided-by-minus-one` | edge | 0 | `i64::MIN / -1` et `i64::MIN % -1` | 1 exécution |
| `013-arith-signed-remainder` | edge | 0 | troncature vers zéro et signe du reste | 1 exécution |
| `014-arith-div-by-zero` | edge | B.3 | division par zéro | 1 exécution |
| `202-arith-the-four-operations` | nominal | B.3 | `ADD`, `SUB`, `MUL`, `DIV`, `MOD`, `NEG` sur de petits opérandes positifs | 1 exécution |
| `203-arith-comparisons` | nominal | B.3 | `EQ`, `LT`, `GT`, `NOT`, chacun rendant `0` ou `1` | 1 exécution |

## `gas`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `015-gas-loop-runs-out` | edge | B.2 | une boucle arrêtée par le budget | 1 exécution, budget 14 |
| `016-gas-exact-exhaustion` | edge | B.2 | une exécution dont le gaz restant vaut exactement zéro | 1 exécution, budget 12, rappels scriptés |
| `017-gas-non-monotonic-table` | edge | B.3 | le coût exact d'une séquence mêlée | 2 exécutions, budget 21 |
| `018-gas-refused-instruction-not-charged` | edge | B.2 | le gaz d'une instruction refusée n'est pas facturé | 1 exécution, budget 10, rappels scriptés |
| `055-gas-reports-what-is-left-after-its-charge` | edge | B.7 | `GAS` se lit après son propre prélèvement | 1 exécution |
| `055b-gas-reports-what-is-left-after-its-charge` | edge | B.7 | `GAS` lu contre la table de gaz non monotone | 1 exécution, rappels scriptés |
| `062-gas-load-and-store-cost-one-less-than-the-popped-forms` | edge | B.3 | `LOAD k`/`STORE k` coûtent un de moins que `LOADX`/`STOREX` | 2 exécutions |
| `212-gas-a-loop-that-watches-its-budget` | nominal | B.7 | une boucle qui consulte `GAS` et s'arrête avant le budget | 1 exécution |

## `jump`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `019-jump-base-is-the-next-instruction` | edge | B.4 | la base du saut est l'instruction suivante | 1 exécution |
| `020-jump-out-of-bounds` | edge | B.4 | une cible au-delà du code | 1 exécution |
| `021-jump-onto-the-last-byte` | edge | B.4 | un saut sur le tout dernier octet du code | 1 exécution |
| `022-jump-tight-loop` | edge | B.4 | une boucle comptée qui finit sur `HALT` | 1 exécution |
| `051-jump-jmpx-target-is-absolute` | edge | B.4 | `JMPX` dépile une cible absolue, pas un déplacement | 1 exécution |
| `051b-jump-jmpx-target-is-absolute` | edge | B.4 | une table de sauts : index lu par `CONSTX`, cible suivie par `JMPX` | 1 exécution, rappels scriptés |
| `052-jump-jmpx-target-out-of-range` | edge | B.4 | une cible dépilée négative est `BadJump`, pas une faute neuve | 1 exécution |
| `052b-jump-jmpx-target-out-of-range` | edge | B.4 | une cible dépilée au-delà du code : le même `BadJump` | 1 exécution |
| `053-jump-jmpx-target-without-the-marker` | edge | B.4 | une cible dépilée doit porter le marqueur | 1 exécution |
| `053b-jump-jmpx-target-without-the-marker` | edge | B.4 | le marqueur se lit sur un octet, même au milieu d'un opérande | 1 exécution |
| `207-jump-if-else-on-the-agent-id` | nominal | B.4 | un programme, deux branches, choisies par la réponse à `SENSE ID` | 2 exécutions, rappels scriptés |
| `208-jump-sum-in-a-loop` | nominal | B.4 | la somme de 1 à 5 par une boucle comptée | 1 exécution |

## `call`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `023-call-stack-overflow` | edge | B.1 | un dix-septième cadre d'appel | 1 exécution |
| `024-call-return-without-a-frame` | edge | B.4 | `RETURN` sans cadre, et un retour sur un octet ordinaire | 1 exécution |
| `054-call-callx-returns-like-call` | edge | B.3 | `CALLX` dépile sa cible et revient comme `CALL` | 1 exécution |
| `054b-call-callx-returns-like-call` | edge | B.3 | `CALLX` sur pile vide : `StackUnderflow` avant tout cadre empilé | 1 exécution |
| `209-call-a-subroutine-twice` | nominal | B.4 | un sous-programme « carré », appelé deux fois | 1 exécution |
| `210-call-a-dispatch-table` | nominal | B.3 | `CALLX` à travers une table de gestionnaires, indexée par la réponse à `SENSE TURN` | 3 exécutions, rappels scriptés |

## `fault`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `026-fault-gas-before-execution` | edge | B.2 | une pile pleine sans gaz restant | 1 exécution, budget 64 |
| `063-fault-store-on-an-empty-stack-underflows` | edge | B.5 | `STORE k` sur pile vide est `StackUnderflow`, jamais `OutOfBounds` | 1 exécution |

## `act`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `032-act-already-acted` | edge | E.4 | un second `ACT` : `AlreadyActed`, levée par la machine sans rappel | 1 exécution, rappels scriptés |
| `060-act-outcome-on-the-stack` | edge | B.3, E.4 | `ACT` empile sa propre issue | 1 exécution, rappels scriptés |

## `degenerate`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `040-degenerate-empty-constant-table` | edge | B.3 | `CONST` dans une table de constantes vide | 1 exécution |

## `bitwise`

| Cas | Nature | Clause | Ce que le cas éprouve | Exécutions |
|---|---|---|---|---|
| `044-bitwise-and-or-xor-bnot` | edge | B.6 | `AND`, `OR`, `XOR` sur le motif en complément à deux | 1 exécution |
| `044b-bitwise-and-or-xor-bnot` | edge | B.6 | `NOT` est logique, `BNOT` est le complément à un | 1 exécution |
| `045-bitwise-shifts-are-logical` | edge | B.6 | `SHR` est logique, et le compte est dépilé en premier | 1 exécution |
| `045b-bitwise-shifts-are-logical` | edge | B.6 | un compte hors de `0..=63` rend `0`, jamais une faute | 1 exécution |
| `049-bitwise-sar-propagates-the-sign` | edge | B.6 | `SAR` est arithmétique là où `SHR` est logique | 1 exécution |
| `049b-bitwise-sar-propagates-the-sign` | edge | B.6 | `SAR` arrondit vers moins l'infini, `DIV` tronque vers zéro | 1 exécution |
| `050-bitwise-sar-past-the-width` | edge | B.6 | un compte de `SAR` hors de `0..=63` remplit de signe, pas de zéros | 1 exécution |
| `050b-bitwise-sar-past-the-width` | edge | B.6 | `SHL` et `SHR` gardent leur règle hors largeur : `0`, quel que soit le signe | 1 exécution |
| `211-bitwise-flags` | nominal | B.6 | poser, tester, basculer des bits ; petits décalages | 1 exécution |
