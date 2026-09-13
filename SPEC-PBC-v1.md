# Palestre — Spécification normative `PBC v1`

**Statut** : normatif. **Version** : `1.9.0`. **Date** : 2026-09-13.

Ce document est le contrat commun à tous les groupes. Toute implémentation conforme doit produire, pour une entrée
donnée, exactement le même résultat d'exécution, le même gaz consommé, les mêmes fautes, la même mémoire finale et les
mêmes séquences de rappels réseau que toute autre implémentation conforme.

En cas de contradiction entre ce document et le sujet (`SUJET.md`), **ce document fait foi**.

Les mots **DOIT**, **NE DOIT PAS**, **DEVRAIT** et **PEUT** ont le sens habituel des documents normatifs.

---

## 0. Conventions générales

* Tous les entiers multi-octets — fichier bytecode, trames réseau de l'annexe K — sont en **gros-boutiste**
  (*big-endian*). Sans exception.
* Le type de valeur unique de la machine est `i64`, complément à deux.
* Toute l'arithmétique est **enveloppante** (*wrapping*). Un dépassement n'est jamais une faute.
* La division tronque vers zéro ; le reste porte le signe du dividende (sémantique de `/` et `%` de Rust sur `i64`, et
  **non** celle de Python).
* `i64::MIN / -1` vaut `i64::MIN` (enveloppant). `i64::MIN % -1` vaut `0`.
* Un hachage est toujours SHA-256, et se note en hexadécimal **minuscule** sur 64 caractères.

---

## Annexe A — Format du fichier `.pbc`

### A.1 Structure

| Décalage | Taille            | Champ         | Contrainte                           |
|----------|-------------------|---------------|--------------------------------------|
| `0`      | 4                 | `magic`       | doit valoir `50 42 43 31` (`"PBC1"`) |
| `4`      | 1                 | `version`     | doit valoir `0x01`                   |
| `5`      | 1                 | `flags`       | doit valoir `0x00` en v1             |
| `6`      | 2                 | `const_count` | `u16`, `0..=4095`                    |
| `8`      | `8 × const_count` | `constants`   | suite de `i64`                       |
| —        | 4                 | `code_len`    | `u32`, `1..=65535`                   |
| —        | `code_len`        | `code`        | octets d'instructions                |

Aucune somme de contrôle. Le fichier **DOIT** faire exactement la taille impliquée par ses champs ; tout octet
excédentaire ou manquant est une erreur.

### A.2 Erreurs de chargement

Toute violation ci-dessus produit `Fault::BadHeader`. Le chargement **NE DOIT PAS** provoquer de `panic`, d'allocation
non bornée, ni de lecture hors limites, quelle que soit la suite d'octets fournie — y compris une suite aléatoire,
tronquée ou construite pour nuire.

### A.3 Note de conception

Une cible de saut **DOIT** porter le marqueur `JUMPDEST` (`0x43`, annexe B.3). Ce qui est regardé est **l'octet lui-même
à l'adresse visée**, sans aucune analyse préalable du code : il n'y a pas de table de destinations valides à construire
au chargement, et décoder à un décalage ne demande jamais de savoir ce qui le précède.

Un saut **PEUT** donc toujours atterrir au milieu d'une instruction, pourvu que l'octet visé vaille `0x43` — fût-il
l'octet de poids faible de l'opérande d'un `PUSHI16 67`. Le décodage reprend à cet octet, qui s'exécute alors comme un
`JUMPDEST` ordinaire.

C'est une divergence **délibérée** d'avec l'EVM pré-EOF, qui exclut de ses destinations valides les octets de données
d'un `PUSH` et paie cette exactitude d'un balayage complet du code. Ici le contrat reste local, et la vérification
statique du bytecode n'en devient pas triviale pour autant — elle reste l'objet d'un des bonus.

---

## Annexe B — Jeu d'instructions et sémantique

### B.1 Machine

| Ressource              | Taille                                     | Dépassement                |
|------------------------|--------------------------------------------|----------------------------|
| Pile de données        | 64 emplacements `i64`                      | `Fault::StackOverflow`     |
| Pile d'appels          | 16 adresses de retour                      | `Fault::CallStackOverflow` |
| Mémoire par agent      | 256 cellules `i64`, indices `0..=255`      | `Fault::OutOfBounds`       |
| Budget de gaz par tour | `gas_budget`, **1000** usuel (`1..=65535`) | `Fault::OutOfGas`          |

La mémoire est **persistante d'un tour à l'autre**. La pile de données et la pile d'appels sont **vidées au début de
chaque tour**. Le compteur ordinal (`pc`) repart de `0` à chaque tour.

### B.2 Cycle d'exécution

À chaque pas, dans cet ordre :

1. Si `pc >= code_len` ⇒ `Fault::PcOutOfRange`.
2. Décoder l'opcode à `pc`. Opcode inconnu ⇒ `Fault::BadOpcode`.
3. Si l'opérande dépasse `code_len` ⇒ `Fault::Truncated`.
4. **Prélever le gaz** de l'instruction. Si le gaz restant est insuffisant ⇒ `Fault::OutOfGas`, sans exécuter
   l'instruction.
5. Exécuter. Toute faute levée ici a déjà consommé son gaz.
6. `pc` avance de la taille de l'instruction, sauf saut réussi.

**Gaz consommé du tour.** Le gaz d'une instruction **refusée** à l'étape 4 n'est pas prélevé : il n'entre pas dans le
`gas_used` du tour. Le gaz d'une instruction **exécutée** l'est toujours, y compris lorsqu'elle lève une faute à l'étape

5. Conséquence : `gas_used` strictement inférieur au budget est le cas ordinaire d'un tour qui s'achève sur
   `OutOfGas` ; l'égalité avec le budget n'y survient que si le reliquat était exactement nul.

L'ordre des étapes 4 et 5 est normatif et observable : un `PUSH` sur une pile pleine, sans gaz restant, doit produire
`OutOfGas` et non `StackOverflow`. `gas_used` et `fault` étant normatifs, une machine qui prélève le gaz
après exécution est détectée même lorsque l'état final est identique.

### B.3 Jeu d'instructions

**Notation de pile — clause à lire deux fois.** `a b -- r` signifie que `b` est au sommet **avant** l'opération, `r` au
sommet **après** l'opération. Le nom le plus proche de `--` est celui du sommet, dans les deux cas : c'est lui qui se
dépile en premier et lui qui s'empile en dernier. L'ordre d'écriture gauche-à-droite n'est **pas** l'ordre de la pile —
un réflexe de lecture naturelle suggère l'inverse, et c'est le piège. Exemple : sur une pile `a b` (`b` au sommet), un
`DROP` appliqué à ce sommet s'écrit `a b -- a` — c'est `b` qui disparaît, pas `a`, alors même que `a` est le nom écrit
en
premier. Toute instruction à deux opérandes de cette annexe se lit ainsi ; la ligne de la table ne le répète pas.

**Les mnémoniques de cette table sont indicatifs, pas normatifs.** Seuls l'octet, l'opérande, le gaz, la pile et la
sémantique font foi. Un rendu étudiant qui écrirait `CONST` autrement, ou pas du tout, reste conforme tant que ses
octets, son gaz et son effet sur la pile le sont.

| Code   | Mnémonique  | Opérande | Gaz | Pile             | Sémantique                                                                                             |
|--------|-------------|----------|-----|------------------|--------------------------------------------------------------------------------------------------------|
| `0x00` | `HALT`      | —        | 0   | `--`             | Fin normale du tour.                                                                                   |
| `0x01` | `CONST k`   | `u16`    | 1   | `-- v`           | `v = constants[k]`. `k >= const_count` ⇒ `Fault::BadConst`.                                            |
| `0x02` | `PUSHI16 n` | `i16`    | 1   | `-- v`           | `v = n` étendu en signe.                                                                               |
| `0x03` | `DROP`      | —        | 1   | `a --`           |                                                                                                        |
| `0x04` | `DUP`       | —        | 1   | `a -- a a`       |                                                                                                        |
| `0x05` | `SWAP`      | —        | 1   | `a b -- b a`     |                                                                                                        |
| `0x06` | `OVER`      | —        | 1   | `a b -- a b a`   |                                                                                                        |
| `0x07` | `ROT`       | —        | 1   | `a b c -- b c a` | rotation de trois                                                                                      |
| `0x08` | `CONSTX`    | —        | 2   | `k -- v`         | `v = constants[k]`, `k` **dépilé**. `k < 0` ou `k >= const_count` ⇒ `Fault::BadConst`.                 |
| `0x10` | `ADD`       | —        | 2   | `a b -- a+b`     | enveloppant                                                                                            |
| `0x11` | `SUB`       | —        | 2   | `a b -- a-b`     | enveloppant                                                                                            |
| `0x12` | `MUL`       | —        | 2   | `a b -- a*b`     | enveloppant                                                                                            |
| `0x13` | `DIV`       | —        | 7   | `a b -- a/b`     | `b == 0` ⇒ `Fault::DivByZero`                                                                          |
| `0x14` | `MOD`       | —        | 7   | `a b -- a%b`     | `b == 0` ⇒ `Fault::DivByZero`                                                                          |
| `0x15` | `NEG`       | —        | 2   | `a -- -a`        | enveloppant                                                                                            |
| `0x20` | `EQ`        | —        | 2   | `a b -- 0\|1`    |                                                                                                        |
| `0x21` | `LT`        | —        | 2   | `a b -- 0\|1`    | comparaison signée `a < b`                                                                             |
| `0x22` | `GT`        | —        | 2   | `a b -- 0\|1`    | comparaison signée `a > b`                                                                             |
| `0x23` | `NOT`       | —        | 2   | `a -- 0\|1`      | `1` si `a == 0`, sinon `0` — **logique**, à ne pas confondre avec `BNOT`                               |
| `0x24` | `AND`       | —        | 2   | `a b -- a&b`     | et bit à bit                                                                                           |
| `0x25` | `OR`        | —        | 2   | `a b -- a\|b`    | ou bit à bit                                                                                           |
| `0x26` | `XOR`       | —        | 4   | `a b -- a^b`     | ou exclusif bit à bit                                                                                  |
| `0x27` | `SHL`       | —        | 3   | `a n -- v`       | décalage **logique** à gauche de `n` bits (B.6)                                                        |
| `0x28` | `SHR`       | —        | 6   | `a n -- v`       | décalage **logique** à droite de `n` bits (B.6)                                                        |
| `0x29` | `BNOT`      | —        | 2   | `a -- !a`        | complément à un — **bit à bit**, à ne pas confondre avec `NOT`                                         |
| `0x2D` | `SAR`       | —        | 6   | `a n -- v`       | décalage **arithmétique** à droite de `n` bits (B.6)                                                   |
| `0x30` | `LOADX`     | —        | 4   | `addr -- v`      | `v = mem[addr]`                                                                                        |
| `0x31` | `STOREX`    | —        | 5   | `v addr --`      | `mem[addr] = v`.                                                                                       |
| `0x32` | `LOAD k`    | `u8`     | 3   | `-- v`           | `v = mem[k]` (`1.9.0`)                                                                                 |
| `0x33` | `STORE k`   | `u8`     | 4   | `v --`           | `mem[k] = v` (`1.9.0`)                                                                                 |
| `0x40` | `JMP d`     | `i16`    | 2   | `--`             | saut relatif                                                                                           |
| `0x41` | `JZ d`      | `i16`    | 2   | `c --`           | saute si `c == 0`                                                                                      |
| `0x42` | `JNZ d`     | `i16`    | 2   | `c --`           | saute si `c != 0`                                                                                      |
| `0x43` | `JUMPDEST`  | —        | 1   | `--`             | marque une cible de saut valide (A.3, B.4) ; aucun autre effet                                         |
| `0x44` | `JMPX`      | —        | 3   | `t --`           | saut à la cible **absolue** dépilée (B.4)                                                              |
| `0x50` | `CALL d`    | `i16`    | 6   | `--`             | empile l'adresse de retour, puis saut relatif                                                          |
| `0x51` | `RETURN`    | —        | 4   | `--`             | dépile l'adresse de retour ; pile vide ⇒ `Fault::CallStackUnderflow`                                   |
| `0x52` | `CALLX`     | —        | 7   | `t --`           | empile l'adresse de retour, puis saut à la cible **absolue** dépilée                                   |
| `0x60` | `SENSE k`   | `u8`     | 8   | `-- v`           | capteur `k` (annexe E) ; inconnu ⇒ `Fault::BadSensor`                                                  |
| `0x61` | `ACT k`     | `u8`     | 12  | `arg -- ok`      | action `k` (annexe E) ; inconnue ⇒ `Fault::BadAction` ; deuxième `ACT` du tour ⇒ `Fault::AlreadyActed` |
| `0x70` | `RAND`      | —        | 5   | `-- v`           | tire une valeur aléatoire `v` fournie par l'hôte/environnement (rappel réseau K.3)                     |
| `0x71` | `GAS`       | —        | 2   | `-- g`           | gaz restant du tour, **son propre coût déjà prélevé** (B.7)                                            |
| `0xF0` | `TRACE`     | —        | 1   | `v --`           | ajoute `v` à la trace du tour ; aucun effet sur l'état                                                 |

Tout autre octet ⇒ `Fault::BadOpcode`.

`TRACE` coûtant `1`, la trace d'un agent compte **au plus `gas_budget` entrées** par tour — soit 1000 sous le budget
usuel, et `65535` au maximum absolu, `gas_budget` étant un `u16`.
Cette borne est ce qui rend tenable la règle de non-allocation dans la boucle d'un tour : le tampon de trace est
dimensionné une fois pour le budget de la partie, jamais pendant le tour. Elle fixe aussi la borne de trame de K.2.

### B.4 Calcul des sauts — attention

L'adresse de saut est **relative au premier octet de l'instruction suivante**, pas à celui de l'instruction de saut :

```
cible = pc + taille_instruction + d
```

Deux conditions, vérifiées dans cet ordre :

* la cible **DOIT** tomber dans `[0, code_len)` ;
* l'octet `code[cible]` **DOIT** valoir `0x43`, l'opcode de `JUMPDEST`.

L'une ou l'autre violée ⇒ `Fault::BadJump`. Une seule chaîne couvre les deux cas, comme l'EVM n'en a qu'une : ce qui est
observable est qu'on n'a **pas** sauté, non la raison du refus.

La seconde condition ne lit qu' **un octet**, celui de la cible (A.3). Une cible valide est donc atteinte quel que soit
son alignement sur une frontière d'instruction, et un `0x43` situé au milieu d'un opérande est une destination légale.

`JMP`, `JZ`, `JNZ`, `CALL`, `JMPX` et `CALLX` y sont soumis — pour `JZ` et `JNZ`, seulement lorsque la condition est
vraie et que le saut a donc lieu. `RETURN` n'y est **pas** soumis : l'adresse qu'il dépile a été empilée par un `CALL`et
désigne l'octet suivant cet appel ; exiger un `JUMPDEST` là reviendrait à imposer un marqueur après chaque appel.

**Cible absolue — `JMPX` et `CALLX`.** Ces deux instructions ne portent pas de déplacement : elles **dépilent** leur
cible, et cette cible est **absolue**. La formule ci-dessus ne s'y applique donc pas, mais les **deux conditions**, si,
mot pour mot — la cible DOIT tomber dans `[0, code_len)`, et l'octet qui s'y trouve DOIT valoir `0x43`. Une valeur
dépilée négative, ou supérieure à `code_len`, viole la première : c'est un `BadJump` ordinaire, et non une faute
nouvelle. `Fault` reste close aux seize entrées de B.5.

La cible d'un saut dépilé est absolue et non relative parce qu'un déplacement dépilé dépendrait de l'adresse de
l'instruction qui le lit : une table de sauts cesserait d'être relogeable, et chaque entrée devrait être recalculée à
l'assemblage — c'est-à-dire qu'on retrouverait l'opérande immédiat en payant une dépile de plus.

### B.5 Liste exhaustive des fautes

`BadHeader`, `BadConst`, `BadOpcode`, `Truncated`, `PcOutOfRange`, `BadJump`, `StackUnderflow`, `StackOverflow`,
`CallStackOverflow`, `CallStackUnderflow`, `OutOfGas`, `DivByZero`, `OutOfBounds`, `BadSensor`, `BadAction`,
`AlreadyActed`.

Ces chaînes exactes sont normatives : elles apparaissent telles quelles dans le champ `fault` des comptes-rendus
d'exécution et protocoles réseau, eux-mêmes normatifs. C'est ce qui rend observable la différence entre `OutOfGas` et
`StackOverflow` sur une pile pleine sans gaz restant — deux machines qui se trompent d'ordre annulent le tour et
restaurent le même état, mais ne consignent pas la même chaîne.

`BadJump` couvre deux refus distincts — cible hors du code, et cible qui ne porte pas le marqueur `JUMPDEST` (B.4). En
distinguer une dix-septième chaîne n'aurait rien ajouté d'observable : dans les deux cas, le tour s'arrête sans avoir
sauté. Il couvre aussi la cible **dépilée** hors de `[0, code_len)` de `JMPX` et `CALLX` : c'est le même refus, pour la
même raison observable.

Les six instructions ajoutées par la `1.3.0` n'introduisent **aucune** chaîne : `BadConst` couvre l'index dépilé de
`CONSTX`, `BadJump` les cibles absolues, `StackUnderflow` les arguments manquants, et `SAR` ne faute jamais. La liste
reste close à seize entrées.

Les deux instructions ajoutées par la `1.9.0`, `LOAD k` et `STORE k`, n'en introduisent pas non plus : leur opérande
`u8` couvre exactement `mem[0..256]`, donc `Fault::OutOfBounds` leur est **inatteignable** par construction, et
`StackOverflow`/`StackUnderflow` couvrent les seuls refus qui leur restent.

`BadHeader` fait exception : c'est une **erreur de chargement**, levée avant que l'exécution ne commence. Elle
n'apparaît
jamais dans les fautes d'un tour en cours — un programme qui ne charge pas n'est pas exécuté (K.3, K.8).

### B.6 Opérations binaires et décalages

`AND`, `OR`, `XOR` et `BNOT` opèrent sur les 64 bits de la représentation en complément à deux. Rien à préciser de
plus : le résultat est un motif de bits, réinterprété en `i64`.

Les décalages, eux, demandent quatre règles, et **aucune** n'est déductible de la table :

* `SHL` et `SHR` sont **logiques**, jamais arithmétiques. Le motif est décalé comme s'il était un `u64`, puis
  réinterprété en `i64`. `SHR` ne propage donc pas le signe : `-1 SHR 1` vaut `9223372036854775807`, et non `-1` ;
* `SAR` est **arithmétique** : il propage le bit de signe. `-1 SAR 1` vaut `-1`, et `-7 SAR 1` vaut `-4` ;
* la pile se lit `a n --` : `a` est la valeur, `n` le nombre de bits (B.3 : `n` au sommet) ;
* un compte hors de `0..=63` — négatif compris — ne faute **jamais**, et les deux familles n'y rendent pas la même
  chose : `SHL` et `SHR` rendent **`0`** ; `SAR` rend le **remplissage de signe**, soit `0` si `a >= 0` et `-1` si
  `a < 0`. La liste de B.5 reste close à seize entrées, et aucun décalage n'interrompt jamais un tour.

```
SHL : v = ((a as u64) << n) as i64      pour 0 <= n <= 63,  sinon 0
SHR : v = ((a as u64) >> n) as i64      pour 0 <= n <= 63,  sinon 0
SAR : v = a >> n                        pour 0 <= n <= 63,  sinon 0 si a >= 0, -1 si a < 0
```

`SHL` et `SHR` opèrent des décalages logiques sur les 64 bits de la valeur réinterprétée en `u64`. `SAR` est l'opération
sur le **nombre signé** et non sur le motif de bits ; les deux cohabitent explicitement plutôt que l'une ne remplace
l'autre, et c'est pour cela que ce sont trois opcodes et non deux.

**La règle de débordement de `SAR` est délibérément la seconde du document.** `SAR(-1, 63)` vaut `-1` ; `SAR(-1, 64)`
valant `0` contredirait le sens de l'opération à la frontière exacte où il ne lui reste plus que le signe à propager.
Un décalage arithmétique qui cesse d'être arithmétique au soixante-quatrième bit n'est pas une simplification.

**`SAR` n'est pas `DIV` par une puissance de deux.** `DIV` tronque **vers zéro** (B.3), `SAR` arrondit **vers moins
l'infini** : `-7 DIV 2` vaut `-3`, `-7 SAR 1` vaut `-4`. Les deux ne coïncident que sur les valeurs positives.

**`NOT` et `BNOT` ne sont pas la même opération.** `NOT` (`0x23`) est logique et rend `0` ou `1` ; `BNOT` (`0x29`) est
le complément à un. `NOT 0` vaut `1`, `BNOT 0` vaut `-1`, et `BNOT 5` vaut `-6`.

### B.7 `GAS` — ce que la machine lit d'elle-même

`GAS` (`0x71`) empile le gaz restant du tour. La valeur empilée est celle qui reste **après** le prélèvement du coût
de `GAS` lui-même, et cette clause est normative.

Elle n'est pas un choix libre : c'est la lecture directe du cycle de B.2, dont l'étape 4 prélève le gaz et l'étape 5
exécute l'instruction. Quand `GAS` s'exécute, ses deux unités sont déjà parties. Sous le budget usuel de `1000`, un
`GAS` placé en `pc = 0` empile donc **`998`**.

```
gas_budget = 1000
pc = 0 :  GAS   ⇒  998
```

C'est le même ordre 4-puis-5 qui fait qu'un `PUSH` sur pile pleine sans gaz restant produit `OutOfGas` et non
`StackOverflow`. Deux implémentations qui s'en écartent divergent exactement de `2`, et `trace` le rend visible.

`GAS` ne faute que par `StackOverflow`, sur une pile déjà pleine. La valeur empilée tient toujours dans un `i64` :
`gas_budget` est un `u16` (`1..=65535`).

---

## Annexe D — Gaz

### D.1 Gaz

La table de gaz est celle de l'annexe B. Elle n'est ni monotone ni intuitive — `STOREX` plus cher que `LOADX`, `SHR`
plus cher que `SHL` — et c'est voulu. Elle est appliquée telle quelle. `STORE`/`LOAD`, la forme à adresse constante
de la `1.9.0`, coûtent un de moins que `STOREX`/`LOADX` : le `PUSHI16` qui disparaît, plus une remise supplémentaire
pour l'accès qui n'a plus rien à dépiler.

Budget par tour : `gas_budget`, au minimum `1` et au maximum `65535` puisqu'il est un `u16`. La valeur usuelle est
`1000`.
Le gaz non consommé n'est pas reporté d'un tour sur l'autre. `HALT` coûte `0`, donc un programme peut toujours s'arrêter
proprement.

---

## Annexe E — Règles des tours et de victoire

### E.2 Déroulement d'un tour

Pour le tour `t` (à partir de `1`), les agents sont traités dans l' **ordre croissant d'identifiant**. Pour chaque agent
vivant :

1. Instantané de l'état de l'agent et du monde.
2. Pile et pile d'appels vidées, `pc = 0`, gaz = `gas_budget` (`1000` usuel).
3. Exécution jusqu'à `HALT` ou faute.
4. **Si faute** : l'état de l'agent et du monde est restauré depuis l'instantané. Ne sont **pas** restaurés : le gaz
   consommé, la trace, et **l'action tentée**. La faute est consignée.
5. Décrément d'énergie du tour : `energy -= 1`. Ce décrément s'applique **même en cas de faute**.
6. Si `energy <= 0` : `alive = 0`, et les ressources portées sont déposées sur la case courante.

### E.3 Capteurs — `SENSE k`

| `k`   | Valeur rendue                                                               |
|-------|-----------------------------------------------------------------------------|
| `0`   | `x` de l'agent                                                              |
| `1`   | `y` de l'agent                                                              |
| `2`   | `energy`                                                                    |
| `3`   | `carried`                                                                   |
| `4`   | quantité de ressource sur la case courante                                  |
| `5`   | numéro du tour en cours                                                     |
| `6`   | identifiant de l'agent                                                      |
| `7`   | distance de Manhattan à l'autre agent ; `-1` s'il est mort                  |
| `8`   | masque des quatre voisins bloqués — voir ci-dessous                         |
| `9`   | masque des quatre voisins portant une ressource — voir ci-dessous           |
| `10`  | `x_max` de la grille                                                        |
| `11`  | `y_max` de la grille                                                        |
| `12`  | `turns_max` de la partie                                                    |
| `13`  | masque des quatre voisins occupés par un agent **vivant** — voir ci-dessous |
| autre | `Fault::BadSensor`                                                          |

Le « tour en cours » de `SENSE 5` est le tour `t` en train de s'exécuter.
La distance de `SENSE 7` est une distance de Manhattan **à vol d'oiseau** : `|x1 - x2| + |y1 - y2|`, ignorant les murs.

#### `SENSE 8` — le masque de voisinage

`SENSE 8` rend un entier de `0` à `15`, un bit par direction, dans l'ordre des directions de `MOVE` (E.4) :

| Bit | Valeur | Direction     | Vaut `1` si                                            |
|-----|--------|---------------|--------------------------------------------------------|
| `0` | `1`    | nord (`y-1`)  | la case voisine est **hors de la grille** ou **murée** |
| `1` | `2`    | est  (`x+1`)  | idem                                                   |
| `2` | `4`    | sud  (`y+1`)  | idem                                                   |
| `3` | `8`    | ouest (`x-1`) | idem                                                   |

Le bord de la grille compte comme un mur. Le masque se lit avec les opérations binaires de B.6.

#### `SENSE 9` — le masque des ressources

`SENSE 9` rend un entier de `0` à `15`, sur la même forme :

| Bit | Valeur | Direction     | Vaut `1` si                                                    |
|-----|--------|---------------|----------------------------------------------------------------|
| `0` | `1`    | nord (`y-1`)  | la case voisine est dans la grille et porte une quantité `> 0` |
| `1` | `2`    | est  (`x+1`)  | idem                                                           |
| `2` | `4`    | sud  (`y+1`)  | idem                                                           |
| `3` | `8`    | ouest (`x-1`) | idem                                                           |

C'est un masque de **présence**, jamais de quantité. Un voisin hors de la grille ou muré porte `0`.

#### Les constantes du monde — `SENSE 10`, `11` et `12`

`SENSE 10` et `SENSE 11` rendent les **index maximaux** de la grille, `x_max` et `y_max` (une grille de 4 colonnes rend
`3`).
`SENSE 12` rend `turns_max`, le nombre total de tours prévu pour la partie. Ces trois valeurs sont constantes pour toute
la partie.

#### `SENSE 13` — le masque des agents

`SENSE 13` rend un entier de `0` à `15`, sur la même forme :

| Bit | Valeur | Direction     | Vaut `1` si                                         |
|-----|--------|---------------|-----------------------------------------------------|
| `0` | `1`    | nord (`y-1`)  | la case voisine est occupée par un agent **vivant** |
| `1` | `2`    | est  (`x+1`)  | idem                                                |
| `2` | `4`    | sud  (`y+1`)  | idem                                                |
| `3` | `8`    | ouest (`x-1`) | idem                                                |

Un agent mort ne compte pas (bit à `0`). Combiné à `SENSE 8`, il permet de prédire avec certitude le succès d'un
déplacement :
si `(SENSE 8 | SENSE 13) & (1 << dir) == 0`, le déplacement vers `dir` réussira.

### E.4 Actions — `ACT k`, argument dépilé, issue rendue

Une seule action réussie **ou échouée** par tour. Un second `ACT` lève `Fault::AlreadyActed`.

| `k`   | Action | Argument                                                      | Effet                                                                                                                                                            |
|-------|--------|---------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `0`   | `MOVE` | `0`=nord (`y-1`), `1`=est, `2`=sud, `3`=ouest ; autre ⇒ échec | déplacement d'une case. Hors grille, case **murée**, ou case occupée par un agent **vivant** ⇒ **échec**. Réussite ⇒ `energy -= 1` en plus du décrément de tour. |
| `1`   | `TAKE` | ignoré                                                        | ramasse `min(ressource_case, 5)` ; ajouté à `carried`, retiré de la case                                                                                         |
| `2`   | `DROP` | quantité                                                      | dépose `min(arg, carried)` sur la case ; `arg < 0` ⇒ **échec**                                                                                                   |
| `3`   | `EAT`  | quantité                                                      | convertit `k = min(arg, carried)` charges : `carried -= k`, `energy += eat_rate × k` ; `arg < 0` ⇒ **échec**                                                     |
| autre | —      | —                                                             | `Fault::BadAction`                                                                                                                                               |

Un **échec** d'action n'est pas une faute : le tour se poursuit normalement. Une **faute** interrompt le tour et
déclenche l'annulation décrite en E.2.

**`ACT k` empile son issue** : `arg -- ok`, `ok` valant `1` en cas de succès, `0` en cas d'échec. Toute autre valeur est
**réservée**.
`ACT` dépile son argument puis empile son issue : il ne peut pas produire `Fault::StackOverflow`.

`EAT` convertit des ressources portées en énergie avec un taux `eat_rate` (défaut : `10` points d'énergie par
ressource).
Comme pour `DROP`, l'argument est borné par `carried` et un argument négatif provoque un échec.

### E.5 Fin de partie & Conditions de victoire

La partie s'arrête au tour `turns_max` (`500` usuel), ou dès qu'il ne reste au plus qu'un agent vivant.
Le test a lieu après le passage de tous les agents du tour : un tour commencé est toujours joué en entier.

Règles de départage déterministes :

1. L'agent avec le plus grand `carried`.
2. À égalité, l'agent avec la plus grande `energy`.
3. À égalité encore, **match nul** (`winner: null`).

L'identifiant ne départage jamais. Deux programmes identiques réalisant le même parcours font match nul.

---

## Annexe H — Versions et évolutions

### H.1 Numérotation

Ce document porte un numéro `MAJEUR.MINEUR.CORRECTIF`, annoncé en tête et repris dans les messages réseau (`READY`,
annexe K).

| Rang        | Ce qui le fait bouger                                                                                  |
|-------------|--------------------------------------------------------------------------------------------------------|
| `MAJEUR`    | des règles incompatibles : un artefact conforme à l'une n'a aucun sens pour l'autre                    |
| `MINEUR`    | tout changement **observable** dans un artefact normatif — un gaz, une faute, un transcript de rappels |
| `CORRECTIF` | une correction éditoriale sans effet observable : formulation, exemple, coquille                       |

**Règle de comparabilité.** Deux artefacts ne se comparent que si leurs `MAJEUR` et `MINEUR` sont égaux ; le
`CORRECTIF` est ignoré.

---

## Annexe K — Protocole d'exécution distante

### K.0 Statut

Cette annexe est **normative**. Deux implémentations conformes **DOIVENT** interopérer dans les deux sens : le client
de l'une contre le serveur de l'autre, avec des trames identiques et le même `RESULT` pour les mêmes entrées.

Elle sert la commande requise pour l'évaluation :

```bash
palestre exec --listen <adresse>
```

Aucun port par défaut n'est imposé.

### K.1 Modèle : la machine sans le monde

Le serveur **exécute lui-même** le programme déposé, dans sa propre machine, mais il ne détient **aucun monde** : ni
grille,
ni ressources, ni second agent. Ce que `SENSE`, `ACT` et `RAND` demanderaient à un monde, le serveur le demande au
**client**,
par un aller-retour de rappel : le client agit comme un environnement d'exécution distant.

Le serveur applique purement les règles de B.1–B.7 à un programme, une mémoire initiale et un budget de gaz fournis par
le
client, et délègue au client chaque interaction avec l'environnement extérieur.

Conséquence directe : **aucun choix n'est fait côté serveur.** Ni carte, ni aléa serveur — `RAND` est délégué au client.

### K.2 Cadrage

Toute trame, dans les deux sens, respecte la structure binaire suivante :

| Décalage | Taille | Champ     | Contrainte                                               |
|----------|--------|-----------|----------------------------------------------------------|
| `0`      | 1      | `type`    | `0x11..=0x1D`, voir table K.3                            |
| `1`      | 4      | `len`     | `u32` **gros-boutiste**, borne selon le sens, ci-dessous |
| `5`      | `len`  | `payload` | Charge utile JSON ou binaire                             |

Bornes normatives de taille :

| Sens             | Borne de `len` | Justification                                                                                               |
|------------------|----------------|-------------------------------------------------------------------------------------------------------------|
| client → serveur | `98307`        | taille maximale d'un `.pbc` selon A.1 : `8 + 8 × 4095 + 4 + 65535`. `SUBMIT` est la seule trame volumineuse |
| serveur → client | `4194304`      | un `RESULT` porte au plus `256 + 65535` entiers `i64` écrits en JSON (mémoire dense de B.1 plus la trace)   |

Une trame annonçant un `len` supérieur **DOIT** être rejetée **avant toute allocation**.
Un `type` hors de `0x11..=0x1D` **DOIT** être rejeté et provoquer une erreur `bad_frame` puis la fermeture de connexion.

### K.3 Messages

| `type` | Nom       | Sens             | Charge utile                                     |
|--------|-----------|------------------|--------------------------------------------------|
| `0x11` | `OPEN`    | client → serveur | JSON — version du protocole, nom optionnel       |
| `0x12` | `READY`   | serveur → client | JSON — spécification, version, bornes du serveur |
| `0x13` | `SUBMIT`  | client → serveur | **octets bruts** du fichier `.pbc`               |
| `0x14` | `SESSION` | serveur → client | JSON — identifiant de session, empreinte, taille |
| `0x15` | `EXEC`    | client → serveur | JSON — session, mémoire initiale, budget         |
| `0x16` | `SENSE`   | serveur → client | JSON — numéro de capteur (rappel, E.3)           |
| `0x17` | `SENSED`  | client → serveur | JSON — valeur rendue, ou faute                   |
| `0x18` | `ACT`     | serveur → client | JSON — action et argument (rappel, E.4)          |
| `0x19` | `ACTED`   | client → serveur | JSON — issue de l'action                         |
| `0x1A` | `RAND`    | serveur → client | JSON — vide `{}` (rappel, K.6)                   |
| `0x1B` | `DREW`    | client → serveur | JSON — valeur tirée                              |
| `0x1C` | `RESULT`  | serveur → client | JSON — mémoire finale, gaz, faute, action, trace |
| `0x1D` | `ERROR`   | serveur → client | JSON — code et message                           |

`SUBMIT` transporte les octets bruts du fichier bytecode `.pbc`, sans encodage intermédiaire.

Schémas JSON des messages :

```json
OPEN     {
  "proto": 1,
  "client": "reference-cli"
}          // "client" est un nom d'affichage optionnel

READY    {
  "proto": 1,
  "spec": "PBC1",
  "spec_version": "1.9.0",
  "sessions_max": 16
}

SUBMIT   <octets bruts du .pbc>

SESSION  {
  "session": "1",
  "pbc_sha256": "…",
  "code_len": 42
}          // "session" est une chaîne opaque : voir K.4. "pbc_sha256" et "code_len" attestent
// du chargement conforme du binaire

EXEC     {
  "session": "1",
  "mem": [
    0,
    0,
    100,
    "… 253 valeurs de plus …"
  ],
  "budget": 1000
}

SENSE    {
  "k": 7
}

SENSED   {
  "value": -1
}          // ou {"fault": "BadSensor"} si le capteur est invalide pour le client

ACT      {
  "kind": "MOVE",
  "arg": 0
}

ACTED    {
  "ok": true
}

RAND     {}

DREW     {
  "value": 42
}

RESULT   {
  "mem": [
    0,
    0,
    98,
    "… 253 valeurs de plus …"
  ],
  "gas_used": 137,
  "fault": null,
  "act": {
    "kind": "MOVE",
    "arg": 0,
    "ok": true
  },
  "trace": [
    1,
    7
  ]
}

ERROR    {
  "code": "bad_frame",
  "message": "session hors du jeu de caractères autorisé"
}
```

`fault` prend l'une des chaînes exactes de B.5, ou `null`.
`SENSED` porte `value` **ou** `fault`, jamais les deux : un `SENSED` qui nomme une faute de chargement (`BadHeader`)
est refusé comme `bad_frame`.
`RESULT.act` prend la forme `{kind, arg, ok}` où `kind` est `"MOVE"`, `"TAKE"`, `"DROP"` ou `"EAT"`.

**`proto` et `spec_version` sont deux choses distinctes** : `proto` versionne le transport (K.2), `spec_version`
versionne les règles d'exécution. Le serveur ne doit pas refuser une connexion sur le seul motif d'une `spec_version`
différente : il annonce sa version dans `READY`.

### K.4 L'identifiant de session est opaque

Sur le fil : `session: String`, de 1 à 64 caractères pris dans `[A-Za-z0-9_-]`, comparé **octet pour octet**. Un champ
hors de ces bornes ou de ce jeu de caractères est un `bad_frame`.

Conséquences normatives :

* Le client **NE DOIT PAS** interpréter un identifiant reçu (ni arithmétique, ni ordre). Il le renvoie **verbatim** dans
  ses `EXEC`.
* Deux `SUBMIT` du même programme peuvent rendre le même identifiant ou deux identifiants distincts.
* Le client ne doit pas réutiliser un identifiant sur une autre connexion TCP.

Machine d'états par connexion TCP :

```
Greeting  ── OPEN ──→  Idle  ⇄  Executing
  (READY)          (SESSION, RESULT/ERROR)
```

* `Greeting` n'accepte que `OPEN` (répond `READY`).
* `Idle` accepte `SUBMIT` (répond `SESSION` ou `ERROR bad_program`) et `EXEC` (bascule en `Executing`).
* `Executing` n'accepte que la réplique exacte du rappel en cours (`SENSED`, `ACTED` ou `DREW`) ; toute autre trame y
  entraîne
  `ERROR unexpected_message` puis la fermeture de la connexion.
* Une session reste chargée au moins jusqu'à la fermeture de sa connexion. Un `EXEC` ne détruit pas la session : un
  programme
  peut être exécuté plusieurs fois.
* Pas de pipelining : une seule exécution active par connexion.

### K.5 Le protocole est la fonction d'exécution, livrée par rappels

`RESULT` est fonction pure de `(programme, mémoire initiale, budget, réponses aux rappels)` :

* **Reproductibilité** : rejouer les mêmes trames rend le même `RESULT`, octet pour octet.
* **Transcript normatif** : deux serveurs conformes émettent exactement la même suite de rappels `SENSE`/`ACT`/`RAND`
  dans le même ordre.

### K.6 Les rappels

Un `SENSE` demande la valeur du capteur `k` (E.3), un `ACT` demande l'exécution de l'action `kind`/`arg` (E.4), un`RAND`
demande un tirage au client. Le client répond respectivement `SENSED`, `ACTED`, `DREW` — strictement dans l'ordre où le
serveur
les pose.

Un `RESULT` n'est émis que si les rappels nécessaires ont tous abouti ; si un rappel échoue ou si le client coupe la
connexion,
celle-ci est close sans émission de `RESULT`.

### K.7 Déterminisme et bornes

Le protocole ne comporte aucun choix non déterministe côté serveur. Tout l'aléa et les interactions proviennent du
client via les rappels.

`RAND` coûtant 5 unités de gaz (D.1), il y a au plus `budget / 5` rappels `RAND` par exécution (200 pour un budget usuel
de 1000).
`sessions_max` (annoncé dans `READY`) borne le nombre de sessions simultanées sur une connexion.

### K.8 Erreurs et robustesse

| `code`               | Cause                                                                |
|----------------------|----------------------------------------------------------------------|
| `bad_frame`          | `len` hors bornes, `type` inconnu, JSON invalide, `session` invalide |
| `bad_proto`          | version de protocole non gérée                                       |
| `bad_program`        | les octets du `SUBMIT` ne passent pas le chargeur de A.1             |
| `too_many_sessions`  | `sessions_max` déjà atteint                                          |
| `no_such_session`    | l'`EXEC` nomme une session inconnue de cette connexion               |
| `bad_memory`         | `mem` n'a pas exactement 256 valeurs                                 |
| `bad_budget`         | `budget` nul ou hors bornes (`1..=65535`)                            |
| `unexpected_message` | trame valide, mais interdite dans l'état courant (K.4)               |

`too_many_sessions` et `no_such_session` **NE ferment PAS** la connexion : la connexion reste utilisable pour d'autres
requêtes.
Tout autre code de ce tableau entraîne l'envoi de `ERROR` puis la fermeture immédiate de la connexion.

### K.9 Hors périmètre en v1

* **Aucune authentification, aucun chiffrement.**
* **Aucune libération explicite de session** : la session vit au moins jusqu'à la fermeture de la connexion TCP.
* **Aucun pipelining** : une seule exécution en cours par connexion TCP.
* **Aucune persistance de mémoire côté serveur entre deux exécutions** : chaque `EXEC` fournit sa mémoire initiale de
  256 `i64`.
