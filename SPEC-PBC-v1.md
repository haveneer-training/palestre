# Palestre — Spécification normative `PBC v1`

**Statut** : normatif. **Version** : `1.9.0`. **Date** : 2026-09-13.

Ce document est le contrat commun à tous les groupes. Toute implémentation conforme doit produire, pour une entrée
donnée, exactement le même résultat d'exécution, le même gaz consommé, les mêmes fautes, la même mémoire finale et les
mêmes séquences de rappels réseau que toute autre implémentation conforme.

En cas de contradiction entre ce document et le sujet (`README.md`), **ce document fait foi**.

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

## <a id="annexe-a"></a>Annexe A — Format du fichier `.pbc`

### <a id="a1"></a>A.1 Structure

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

### <a id="a2"></a>A.2 Erreurs de chargement

Toute violation ci-dessus produit `Fault::BadHeader`. Le chargement **NE DOIT PAS** provoquer de `panic`, d'allocation
non bornée, ni de lecture hors limites, quelle que soit la suite d'octets fournie — y compris une suite aléatoire,
tronquée ou construite pour nuire.

### <a id="a3"></a>A.3 Note de conception

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

## <a id="annexe-b"></a>Annexe B — Jeu d'instructions et sémantique

### <a id="b1"></a>B.1 Machine

| Ressource              | Taille                                     | Dépassement                |
|------------------------|--------------------------------------------|----------------------------|
| Pile de données        | 64 emplacements `i64`                      | `Fault::StackOverflow`     |
| Pile d'appels          | 16 adresses de retour                      | `Fault::CallStackOverflow` |
| Mémoire par agent      | 256 cellules `i64`, indices `0..=255`      | `Fault::OutOfBounds`       |
| Budget de gaz par tour | `gas_budget`, **1000** usuel (`1..=65535`) | `Fault::OutOfGas`          |

La mémoire est **persistante d'un tour à l'autre**. La pile de données et la pile d'appels sont **vidées au début de
chaque tour**. Le compteur ordinal (`pc`) repart de `0` à chaque tour.

### <a id="b2"></a>B.2 Cycle d'exécution

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

### <a id="b3"></a>B.3 Jeu d'instructions

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
| `0x70` | `RAND`      | —        | 5   | `-- v`           | tire une valeur aléatoire `v` fournie par l'hôte/environnement (rappel réseau K.6)                     |
| `0x71` | `GAS`       | —        | 2   | `-- g`           | gaz restant du tour, **son propre coût déjà prélevé** (B.7)                                            |
| `0xF0` | `TRACE`     | —        | 1   | `v --`           | ajoute `v` à la trace du tour ; aucun effet sur l'état                                                 |

Tout autre octet ⇒ `Fault::BadOpcode`.

`TRACE` coûtant `1`, la trace d'un agent compte **au plus `gas_budget` entrées** par tour — soit 1000 sous le budget
usuel, et `65535` au maximum absolu, `gas_budget` étant un `u16`.
Cette borne est ce qui rend tenable la règle de non-allocation dans la boucle d'un tour : le tampon de trace est
dimensionné une fois pour le budget de la partie, jamais pendant le tour. Elle fixe aussi la borne de trame de K.2.

### <a id="b4"></a>B.4 Calcul des sauts — attention

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

### <a id="b5"></a>B.5 Liste exhaustive des fautes

`BadHeader`, `BadConst`, `BadOpcode`, `Truncated`, `PcOutOfRange`, `BadJump`, `StackUnderflow`, `StackOverflow`,
`CallStackOverflow`, `CallStackUnderflow`, `OutOfGas`, `DivByZero`, `OutOfBounds`, `BadSensor`, `BadAction`,
`AlreadyActed`.

Ces chaînes exactes sont normatives : elles apparaissent telles quelles dans le champ `fault` des comptes-rendus
d'exécution et protocoles réseau, eux-mêmes normatifs. C'est ce qui rend observable la différence entre `OutOfGas` et
`StackOverflow` sur une pile pleine sans gaz restant — deux machines qui se trompent d'ordre annulent le tour et
restaurent le même état, mais ne consignent pas la même chaîne.

`BadJump` couvre deux refus distincts — cible hors du code, et cible qui ne porte pas le marqueur `JUMPDEST` (B.4). En
distinguer une dix-septième chaîne n'aurait rien ajouté d'observable : dans les deux cas, le tour s'arrête sans avoir
sauté. Il couvre aussi, la cible **dépilée** hors de `[0, code_len)` de `JMPX` et `CALLX` : c'est le
même refus, pour la même raison observable.

Les six instructions ajoutées par la `1.3.0` n'introduisent **aucune** chaîne : `BadConst` couvre l'index dépilé de
`CONSTX`, `BadJump` les cibles absolues, `StackUnderflow` les arguments manquants, et `SAR` ne faute jamais. La liste
reste close à seize entrées.

Les deux instructions ajoutées par la `1.9.0`, `LOAD k` et `STORE k`, n'en introduisent pas non plus : leur opérande
`u8` couvre exactement `mem[0..256]`, donc `Fault::OutOfBounds` leur est **inatteignable** par construction, et
`StackOverflow`/`StackUnderflow` couvrent les seuls refus qui leur restent.

`BadHeader` fait exception : c'est une **erreur de chargement**, levée avant que l'exécution ne commence. Elle
n'apparaît jamais dans les fautes d'un tour en cours — un programme qui ne charge pas n'est pas exécuté (K.3, K.8).

### <a id="b6"></a>B.6 Opérations binaires et décalages

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

`SHL` et `SHR` opèrent des décalages logiques sur les 64 bits de la valeur réinterprétée en `u64`. `SAR` est
l'opération sur le **nombre signé** et non sur le motif de bits ; les deux cohabitent explicitement plutôt que l'une ne
remplace l'autre, et c'est pour cela que ce sont trois opcodes et non deux.

**La règle de débordement de `SAR` est délibérément la seconde du document.** `SAR(-1, 63)` vaut `-1` ; `SAR(-1, 64)`
valant `0` contredirait le sens de l'opération à la frontière exacte où il ne lui reste plus que le signe à propager.
Un décalage arithmétique qui cesse d'être arithmétique au soixante-quatrième bit n'est pas une simplification.

**`SAR` n'est pas `DIV` par une puissance de deux.** `DIV` tronque **vers zéro** (B.3), `SAR` arrondit **vers moins
l'infini** : `-7 DIV 2` vaut `-3`, `-7 SAR 1` vaut `-4`. Les deux ne coïncident que sur les valeurs positives.

**`NOT` et `BNOT` ne sont pas la même opération.** `NOT` (`0x23`) est logique et rend `0` ou `1` ; `BNOT` (`0x29`) est
le complément à un. `NOT 0` vaut `1`, `BNOT 0` vaut `-1`, et `BNOT 5` vaut `-6`.

### <a id="b7"></a>B.7 `GAS` — ce que la machine lit d'elle-même

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

## <a id="annexe-d"></a>Annexe D — Gaz

### <a id="d1"></a>D.1 Gaz

La table de gaz est celle de l'annexe B. Elle n'est ni monotone ni intuitive — `STOREX` plus cher que `LOADX`, `SHR`
plus cher que `SHL` — et c'est voulu. Elle est appliquée telle quelle. `STORE`/`LOAD`, la forme à adresse constante
de la `1.9.0`, coûtent un de moins que `STOREX`/`LOADX` : le `PUSHI16` qui disparaît, plus une remise supplémentaire
pour l'accès qui n'a plus rien à dépiler.

Budget par tour : `gas_budget`, au minimum `1` et au maximum `65535` puisqu'il est un `u16`. La valeur usuelle est
`1000`. Le gaz non consommé n'est pas reporté d'un tour sur l'autre. `HALT` coûte `0`, donc un programme peut toujours
s'arrêter proprement.

---

## <a id="annexe-e"></a>Annexe E — Règles des tours et de victoire

### <a id="e2"></a>E.2 Déroulement d'un tour

Pour le tour `t` (à partir de `1`), les agents sont traités dans l' **ordre croissant d'identifiant**. Pour chaque agent
vivant :

1. Instantané de l'état de l'agent et du monde.
2. Pile et pile d'appels vidées, `pc = 0`, gaz = `gas_budget` (`1000` usuel).
3. Exécution jusqu'à `HALT` ou faute.
4. **Si faute** : l'état de l'agent et du monde est restauré depuis l'instantané. Ne sont **pas** restaurés : le gaz
   consommé, la trace, et **l'action tentée**. La faute est consignée.
5. Décrément d'énergie du tour : `energy -= 1`. Ce décrément s'applique **même en cas de faute**.
6. Si `energy <= 0` : `alive = 0`, et les ressources portées sont déposées sur la case courante.

### <a id="e3"></a>E.3 Capteurs — `SENSE k`

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

La distance de `SENSE 7` est une distance de Manhattan **à vol d'oiseau**: `|x1 - x2| + |y1 - y2|`, ignorant les murs.

#### `SENSE 8` — le masque de voisinage

`SENSE 8` rend un entier de `0` à `15`, un bit par direction, **dans l'ordre des directions de `MOVE`** (E.4) :

| Bit | Valeur | Direction     | Vaut `1` si                                            |
|-----|--------|---------------|--------------------------------------------------------|
| `0` | `1`    | nord (`y-1`)  | la case voisine est **hors de la grille** ou **murée** |
| `1` | `2`    | est  (`x+1`)  | idem                                                   |
| `2` | `4`    | sud  (`y+1`)  | idem                                                   |
| `3` | `8`    | ouest (`x-1`) | idem                                                   |

Le bord de la grille compte donc comme un mur. Le masque se lit avec les opérations binaires de B.6.
Un agent au coin nord-ouest d'un plateau sans le moindre mur lit `9`, soit `0b1001`.

Ce que le masque **ne dit pas** : la présence de l'autre agent. `MOVE` échoue aussi sur une case occupée par un agent
vivant (E.4), et cette cause-là est mobile ; le masque ne décrit que le plateau, qui ne bouge pas. Un bit à `0` promet
donc qu'il n'y a ni bord ni mur, jamais que le déplacement réussira — `SENSE 13` couvre la donnée mobile qui manque
ici (si `(SENSE 8 | SENSE 13) & (1 << dir) == 0`, le déplacement vers `dir` réussira).



Sur un plateau sans mur, `SENSE 8` reste utile et défini : il rend le voisinage de bord. Il n'y a pas de version du
capteur qui n'existerait qu'en présence de murs.

Le masque se lit avec les opérations binaires de B.6 : `SENSE 8`, `PUSHI16 2`, `AND` teste l'est pour onze unités de
gaz,
et chaque direction suivante en coûte quatre — `DUP`, `PUSHI16`, `AND` — tant que le masque est conservé sous la pile.
Sans `AND`, il faudrait un `DIV` et un `MOD` par direction, soit quatorze unités : c'est la raison d'être de B.6.

#### `SENSE 9` — le masque des ressources

`SENSE 9` rend un entier de `0` à `15`, sur **exactement la même forme** que `SENSE 8` : un bit par direction, dans
l'ordre des directions de `MOVE`, bit `0` nord, `1` est, `2` sud, `3` ouest.

| Bit | Valeur | Direction     | Vaut `1` si                                                    |
|-----|--------|---------------|----------------------------------------------------------------|
| `0` | `1`    | nord (`y-1`)  | la case voisine est dans la grille et porte une quantité `> 0` |
| `1` | `2`    | est  (`x+1`)  | idem                                                           |
| `2` | `4`    | sud  (`y+1`)  | idem                                                           |
| `3` | `8`    | ouest (`x-1`) | idem                                                           |

C'est un masque de **présence**, jamais de quantité : il dit *s'il y a*, pas *combien il y a*. Un agent qui choisit où
aller n'a besoin que de la présence — la quantité, il la lira avec `SENSE 4` une fois arrivé, et `TAKE` plafonne de
toute façon à cinq (E.4). Quatre capteurs directionnels rendant la quantité auraient coûté trente-deux unités de gaz
là où le masque en coûte huit.

Un voisin **hors de la grille** porte `0`. Un voisin **muré** porte `0` lui aussi, et sans que la question se pose : une
case murée ne peut pas porter de ressource. Ce masque n'a donc, contrairement à `SENSE 8`, aucune raison de
confondre le bord et le mur — les deux disent `0` pour le même motif, il n'y a rien là.

Ce que le masque **ne dit pas** : la présence de l'autre agent. Comme pour `SENSE 8`, un bit à `1` promet une
ressource, jamais que `MOVE` réussira — la case peut être occupée par un agent vivant (E.4). Le plateau ne bouge pas,
l'adversaire si ; `SENSE 13` le dit.

#### Les constantes du monde — `SENSE 10`, `11` et `12`

`SENSE 10` et `SENSE 11` rendent les **index maximaux** de la grille, `x_max` et `y_max`, jamais une largeur ni une
hauteur : une grille de quatre colonnes rend `3`.

`SENSE 12` rend `turns_max`, le nombre total de tours prévu pour la partie — le dernier tour que la partie
peut jouer, non le nombre de tours restants. À ne pas confondre avec `SENSE 5`, qui rend le tour **en cours** : les
deux noms se ressemblent et les deux valeurs ne se remplacent pas. Le nombre de tours restants s'écrit
`SENSE 12`, `SENSE 5`, `SUB`.

Ces trois valeurs sont **constantes pour toute la partie**. Un programme les lit une fois et les range en mémoire ;
c'est pour cela qu'elles sont trois capteurs et non un mot empaqueté à décoder à chaque usage.

#### `SENSE 13` — le masque des agents

`SENSE 13` rend un entier de `0` à `15`, sur **exactement la même forme** que `SENSE 8` et `SENSE 9` : un bit par
direction, dans l'ordre des directions de `MOVE`, bit `0` nord, `1` est, `2` sud, `3` ouest.

| Bit | Valeur | Direction     | Vaut `1` si                                         |
|-----|--------|---------------|-----------------------------------------------------|
| `0` | `1`    | nord (`y-1`)  | la case voisine est occupée par un agent **vivant** |
| `1` | `2`    | est  (`x+1`)  | idem                                                |
| `2` | `4`    | sud  (`y+1`)  | idem                                                |
| `3` | `8`    | ouest (`x-1`) | idem                                                |

C'est le complément que `SENSE 8` et `SENSE 9` annonçaient chacun ne pas donner : la présence de l'autre agent.
`SENSE 8` dit le bord et le mur, statiques ; `SENSE 9` dit la ressource, statique elle aussi tant que personne ne la
ramasse ; `SENSE 13` dit l'adversaire, seule donnée du triptyque qui **bouge** d'un tour à l'autre. Combiné à
`SENSE 8`, il rend `MOVE` entièrement prévisible avant toute tentative : `SENSE 8 | SENSE 13`, lu bit à bit, promet
qu'une direction à `0` réussira.

Un agent **mort** ne compte pas : son bit reste à `0`, exactement comme s'il n'était pas là. C'est la même réserve
que celle de `SENSE 7`.

### <a id="e4"></a>E.4 Actions — `ACT k`, argument dépilé, issue rendue

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

**`ACT k` empile son issue** : `arg -- ok`, `ok` valant `1` en cas de succès, `0` en cas d'échec. Toute autre valeur
est **réservée** : elle ne peut être ni produite ni interprétée par ce document. `ACT` dépile son argument puis
empile son issue, sans jamais empiler sans avoir dépilé — un `ACT` ne peut donc pas produire `Fault::StackOverflow`,
quel que soit le remplissage de la pile avant l'instruction.

Les trois causes d'échec de `MOVE` — bord, mur, agent vivant — se **prédisent** avant toute tentative :
`SENSE 8` dit le bord et le mur, `SENSE 13` dit l'agent, et la combinaison `SENSE 8 | SENSE 13` promet qu'une
direction dont le bit vaut `0` réussira — la prédiction ne peut pas périmer, puisque E.2 traite les agents en
séquence et que rien d'autre que l'agent courant ne bouge pendant son propre tour. `SENSE 7`, qui ne donne qu'une
distance, jamais une direction, ne prédit rien de tout cela. Un mur n'a pas d'effet sur `TAKE`, `DROP` ni `EAT` : un
agent n'est jamais sur une case murée, donc la question ne se pose pas.

`ACT k` avec un `k` hors de `0..=3` lève `Fault::BadAction` et ne produit **aucune** action. Une direction invalide
de `MOVE`, en revanche, est un échec d'action ordinaire (`ok` vaut `0`).

`EAT` est la seule source d'énergie de la partie, et la seule opération qui retire définitivement de la ressource du
plateau. Le taux est **`eat_rate`** : `eat_rate` points d'énergie par charge convertie, `10` par défaut. Comme le
vainqueur se départage d'abord sur `carried`, convertir reste un **achat de survie payé en score** quel que soit le
taux : c'est le seul arbitrage économique du jeu, et `eat_rate` en règle le prix. Le clampage
`min(arg, carried)` et l'échec sur `arg < 0` sont exactement ceux de `DROP`, et ne dépendent pas du taux — le taux
multiplie l'énergie gagnée, jamais la ressource consommée.

### <a id="e5"></a>E.5 Fin de partie

La partie s'arrête au tour `turns_max` (`500` par défaut), ou dès qu'il ne reste au plus qu'un agent vivant. Le test a
lieu **après** le passage de tous les agents du tour : un tour commencé est toujours joué en entier.

Règles de départage déterministes :

1. L'agent avec le plus grand `carried`.
2. À égalité, l'agent avec la plus grande `energy`.
3. À égalité encore, **match nul** (`winner: null`).

L'identifiant ne départage jamais. Deux programmes identiques réalisant le même parcours font match nul.

---

## <a id="annexe-h"></a>Annexe H — Versions et évolutions

### <a id="h1"></a>H.1 Numérotation

Ce document porte un numéro `MAJEUR.MINEUR.CORRECTIF`, annoncé en tête et repris dans les messages réseau (`READY`,
annexe K).

| Rang        | Ce qui le fait bouger                                                                                  |
|-------------|--------------------------------------------------------------------------------------------------------|
| `MAJEUR`    | des règles incompatibles : un artefact conforme à l'une n'a aucun sens pour l'autre                    |
| `MINEUR`    | tout changement **observable** dans un artefact normatif — un gaz, une faute, un transcript de rappels |
| `CORRECTIF` | une correction éditoriale sans effet observable : formulation, exemple, coquille                       |

**Règle de comparabilité.** Deux artefacts ne se comparent que si leurs `MAJEUR` et `MINEUR` sont égaux ; le
`CORRECTIF` est ignoré — c'est ce qui permet de corriger une phrase sans périmer un corpus. Comparer deux artefacts de
versions mineures différentes n'est pas un échec de conformité, c'est une comparaison dépourvue de sens : un outil
**DOIT** la refuser et nommer les deux versions, plutôt qu'énumérer des divergences de champs.

Le coût est assumé : tout changement observable de ce document périme les `expected.json` produits avant lui. C'est
précisément le but. Sans numéro dans les sorties, rien ne distingue un corpus régénéré d'un corpus ancien, et deux
implémentations correctes contre deux versions différentes se ressemblent trait pour trait.

### <a id="h2"></a>H.2 La version 2

La version distribuée est gelée : une fois le sujet remis, elle ne bouge plus. Une version `2.0.0` **existe** et sera
révélée en soutenance : elle modifie ou ajoute un petit nombre d'éléments de ce document. Concevez pour que ces
changements soient localisés — à commencer par le numéro lui-même, qui ne devrait exister qu'à **un seul endroit** de
votre code.

---

## <a id="annexe-k"></a>Annexe K — Protocole d'exécution distante

### <a id="k0"></a>K.0 Statut

Cette annexe est **normative**. Deux implémentations conformes **DOIVENT** interopérer dans les deux sens : le client
de l'une contre le serveur de l'autre, avec des trames identiques et le même `RESULT` pour les mêmes entrées.

Elle sert la commande requise pour l'évaluation :

```bash
palestre exec --listen <adresse>
```

Aucun port par défaut n'est imposé.

### <a id="k1"></a>K.1 Modèle : la machine sans le monde

Le serveur **exécute lui-même** le programme déposé, dans sa propre machine, mais il ne détient **aucun monde** : ni
grille, ni ressources, ni second agent. Ce que `SENSE`, `ACT` et `RAND` demanderaient à un monde, le serveur le demande
au **client**, par un aller-retour de rappel : le client agit comme un environnement d'exécution distant.

C'est la fonction pure `(bytecode, mémoire initiale, budget) → (mémoire finale, gaz consommé, faute)`, augmentée des
trois rappels qui la rendent capable d'exécuter un programme qui *sent* et *agit*, sans lui fournir de monde à sentir
ni d'action à accomplir : c'est le client qui répond, avec ce qu'il veut.

Conséquence directe : **aucun choix n'est fait côté serveur.** Ni carte, ni aléa serveur — `RAND` est délégué au
client. Le serveur ne fait qu'appliquer B.1–B.7 à un programme, une mémoire et un budget qu'il n'a pas choisis.

### <a id="k2"></a>K.2 Cadrage

Toute trame, dans les deux sens, respecte la structure binaire suivante :

| Décalage | Taille | Champ     | Contrainte                                               |
|----------|--------|-----------|----------------------------------------------------------|
| `0`      | 1      | `type`    | `0x11..=0x1D`, voir table K.3                                  |
| `1`      | 4      | `len`     | `u32` **gros-boutiste**, borne selon le sens, ci-dessous |
| `5`      | `len`  | `payload` | charge utile : JSON, ou octets bruts pour `SUBMIT`       |

Bornes normatives de taille :

| Sens             | Borne de `len` | D'où elle vient                                                                                             |
|------------------|----------------|-------------------------------------------------------------------------------------------------------------|
| client → serveur | `98307`        | taille maximale d'un `.pbc` selon A.1 : `8 + 8 × 4095 + 4 + 65535`. `SUBMIT` est la seule trame volumineuse |
| serveur → client | `4194304`      | un `RESULT` porte au plus `256 + 65535` entiers `i64` écrits en JSON (mémoire dense de B.1 plus la trace)   |

Une trame annonçant un `len` supérieur **DOIT** être rejetée **avant toute allocation**.
Un `type` hors de `0x11..=0x1D` **DOIT** être rejeté et provoquer une erreur `bad_frame` puis la fermeture de connexion.

### <a id="k3"></a>K.3 Messages

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

`SUBMIT` transporte les octets bruts du fichier `.pbc`, sans encodage intermédiaire.

Chaque réplique porte le **passé** du verbe de sa requête — `SUBMIT` reçoit `SESSION`, `EXEC` reçoit finalement
`RESULT`, `SENSE`/`ACT`/`RAND` reçoivent `SENSED`/`ACTED`/`DREW` : une réplique de mauvais genre se détecte à l'octet
de type, avant même d'inspecter son JSON.

```json
OPEN     {
  "proto": 1,
  "client": "reference-cli"
}          // "client" est un nom d'affichage optionnel, sans effet sur les règles
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
}          // "session" est une chaîne opaque : voir K.4. "pbc_sha256" et "code_len" sont un
// checksum de conformité, jamais l'identifiant lui-même
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
}          // ou {"fault": "BadSensor"} si le capteur n'existe pas pour le client
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

`fault` prend l'une des chaînes exactes de B.5, ou `null`. `SENSED` porte `value` **ou** `fault`, jamais les deux :
un `SENSED` qui nomme une faute de **chargement** (telle `BadHeader`) est refusé comme `bad_frame`, puisqu'un programme
déjà chargé ne peut plus lever ce genre de faute. `RESULT.act` prend la forme `{kind, arg, ok}`, où `kind` est `"MOVE"`,
`"TAKE"`, `"DROP"` ou `"EAT"`, et vaut `null` si aucune action n'a été tentée : un `ACT` qui lève `BadAction` n'en
consigne aucune.

**`proto` et `spec_version` restent deux choses distinctes** : `proto` versionne ce cadrage et cet
alphabet, `spec_version` versionne les règles d'exécution (B, D, l'absence de monde ne changeant rien à B.1–B.7). Le
serveur **NE DOIT PAS** refuser une connexion sur le seul motif d'une version de spécification différente — il
**annonce** une seule version dans `READY`, jamais une liste.

### <a id="k4"></a>K.4 L'identifiant de session est opaque

Sur le fil : `session: String`, de 1 à 64 caractères pris dans `[A-Za-z0-9_-]`, comparé **octet pour octet**. Un champ
hors de ces bornes ou de ce jeu de caractères est un `bad_frame`.

Le type est une chaîne, et non un entier, précisément pour laisser le choix à l'implémentation : un serveur **PEUT**
numéroter en décimal à partir de `1`, rendre l'empreinte du programme, un jeton opaque, ou toute autre valeur de son
choix. Conséquences normatives :

* un client **NE DOIT PAS** interpréter un identifiant reçu — ni arithmétique, ni ordre, ni densité supposée. Il le
  renvoie **verbatim** dans ses `EXEC` ;
* deux `SUBMIT` du **même** programme **PEUVENT** rendre le même identifiant (un serveur indexé par empreinte) ou deux
  identifiants distincts (un serveur à compteur). Les deux sont conformes, et un client ne doit dépendre d'aucun des
  deux ;
* un client **NE DOIT PAS** réutiliser un identifiant sur une autre connexion ; un serveur, lui, n'est **pas** tenu de
  le refuser — c'est cette asymétrie qui laisse conforme un serveur sans état, indexé par empreinte.

`SESSION.pbc_sha256` n'est donc **pas** l'identifiant : c'est un **checksum de conformité** rendu au client, qui lui
permet de vérifier que le serveur a chargé les octets qu'il croyait envoyer. `code_len` joue le même rôle pour la
taille. Un serveur qui se sert de cette empreinte
comme identifiant de session est un cas particulier légal, pas le modèle.

Machine d'états, par connexion :

```
Greeting  ── OPEN ──→  Idle  ⇄  Executing
  (READY)          (SESSION, RESULT/ERROR)
```

* `Greeting` n'accepte que `OPEN` (répond `READY`).
* `Idle` accepte `SUBMIT` (répond `SESSION` ou `ERROR bad_program`) et `EXEC` (bascule en `Executing`).
* `Executing` n'accepte que la réplique exacte du rappel en cours (`SENSED`, `ACTED` ou `DREW`) ; toute autre trame y
  entraîne `ERROR unexpected_message` puis la fermeture de la connexion.
* Une session reste chargée au moins jusqu'à la fermeture de sa connexion. Un `EXEC` ne détruit pas la session : un
  programme peut être exécuté plusieurs fois.
* Pas de pipelining : une seule exécution active par connexion; le parallélisme s'obtient par une seconde connexion.

### <a id="k5"></a>K.5 Le protocole est la fonction d'exécution, livrée par rappels

`RESULT` est fonction de `(P, M₀, b, R₁ … R_k)` — le programme, la mémoire initiale, le budget, et les réponses aux
rappels, dans l'ordre où ils sont posés — et de rien d'autre : pas d'horloge, pas d'état serveur, pas de PRNG serveur.

Conséquences :

* **reproductibilité** : rejouer les mêmes trames rend le même `RESULT`, octet pour octet ;
* **le transcript est normatif** : deux serveurs conformes, pour les mêmes `P`, `M₀`, `b` et les mêmes réponses aux
  rappels, émettent la même suite de `SENSE`/`ACT`/`RAND`, dans le même ordre — pas seulement le même résultat final.

### <a id="k6"></a>K.6 Les rappels

Un `SENSE` demande la valeur du capteur `k` (E.3), un `ACT` demande l'exécution de l'action `kind`/`arg` (E.4), un
`RAND` demande un tirage au client. Le client répond respectivement `SENSED`, `ACTED`, `DREW` — jamais dans un autre
ordre que celui dans lequel le serveur les pose, puisqu'une seule exécution est en vol par connexion (K.4).

> Un `RESULT` n'est émis que si les rappels nécessaires ont tous abouti ; une exécution dont un rappel échoue produit
> une fermeture de connexion, pas un résultat.

### <a id="k7"></a>K.7 Déterminisme et bornes

Le protocole ne comporte **aucun** choix non déterministe côté serveur : rien ici ne dépend du réseau, le client
fournit tout, y compris le hasard.

`RAND` coûte 5 unités de gaz (D.1), donc au plus `budget / 5` allers-retours de ce type par exécution — 200 au budget
usuel de `1000`. `sessions_max`, annoncé dans `READY`, borne l'accumulation de sessions ouvertes sur une connexion,
comme la borne de trame de K.2 borne l'allocation d'une trame.

### <a id="k8"></a>K.8 Erreurs et robustesse

| `code`               | Cause                                                                 |
|----------------------|-----------------------------------------------------------------------|
| `bad_frame`          | `len` hors bornes, `type` inconnu, JSON invalide, `session` mal formé |
| `bad_proto`          | version de protocole non gérée                                        |
| `bad_program`        | les octets du `SUBMIT` ne passent pas le chargeur de A.1              |
| `too_many_sessions`  | `sessions_max` déjà atteint                                           |
| `no_such_session`    | l'`EXEC` nomme une session inconnue de cette connexion                |
| `bad_memory`         | `mem` n'a pas exactement 256 valeurs                                  |
| `bad_budget`         | `budget` nul ou hors bornes (`1..=65535`)                             |
| `unexpected_message` | trame valide, mais interdite dans l'état courant (K.4)                |

Il n'existe **délibérément pas** de code `bad_spec_version` : le serveur annonce une seule
version, le client décide. Il n'existe pas non plus de code pour un rappel resté sans réponse : un client qui se tait
est une connexion qui s'en va, pas une erreur applicative à nommer.

`too_many_sessions` et `no_such_session` **NE ferment PAS** la connexion : ce sont des refus d'une demande précise,
une session existante reste utilisable, une nouvelle tentative de `SUBMIT` ou d'`EXEC` reste possible. Tout autre code
de ce tableau ferme la connexion après l'`ERROR`. Une trame tronquée par une coupure n'est pas une
erreur applicative : la connexion est simplement close, sans `ERROR`.

### <a id="k9"></a>K.9 Hors périmètre en v1

* **Aucune authentification, aucun chiffrement** — le serveur ne détient rien qui vaille d'être protégé : ni monde, ni
  score, ni identité persistante.
* **Aucune libération explicite de session.** Une session vit au moins jusqu'à la fermeture de sa connexion (K.4) ;
  la question de sa survie au-delà relève de l'implémentation, pas du protocole.
* **Aucun pipelining.** Une exécution en vol par connexion ; le parallélisme s'obtient par plusieurs connexions.
* **Aucune observation pas à pas.** Le rappel expose le nécessaire (`SENSE`, `ACT`, `RAND`) ; il n'expose ni
  l'instruction
  courante ni la pile — `RESULT` ne transporte donc pas non plus la pile de fin, qui n'est pas un champ normatif.
* **Aucune notion de monde, d'arène ou de second programme** : le serveur n'en connaît aucune.
* **Aucune persistance de mémoire côté serveur au-delà d'une exécution.** Chaque `EXEC` fournit sa propre mémoire
  initiale ; le serveur ne conserve pas la mémoire finale d'une exécution pour la suivante.
