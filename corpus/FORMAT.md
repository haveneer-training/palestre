# Format des cas de test

Chaque cas éprouve une machine **seule**, exécutée comme l'annexe K l'exécute : un programme, une mémoire initiale, un
budget, et les réponses aux rappels `SENSE`, `ACT` et `RAND`. Pas de monde, pas de second programme. Un cas se rejoue
donc contre tout serveur `palestre exec --listen <hôte:port>`.

## Rangement

Les cas sont rangés par nature :

* `edge/` — **cas limites** : chacun vise une clause qu'une implémentation pressée rate. La plupart fautent exprès,
  pas tous ;
* `nominal/` — **usages ordinaires** et valides : aucune exécution n'y faute.

Un cas est un répertoire nommé `NNN-catégorie-intitulé`. Le numéro a trois chiffres, éventuellement suivi d'un `b`
pour le second programme d'un même sujet. La catégorie dit quelle partie de la spécification est éprouvée : `header`,
`decode`, `stack`, `arith`, `gas`, `jump`, `call`, `fault`, `act`, `bitwise` ou `degenerate`.

```
016-gas-exact-exhaustion/
    case.toml       ce que le cas éprouve, et l'entrée de chaque exécution
    program.pbc     le programme, au format de l'annexe A
    expected.json   ce que la machine doit demander et rendre, exécution par exécution
```

## `case.toml`

```toml
description = "an execution whose remaining gas is exactly zero"
category = "gas"
clause = "B.2"

[[exec]]
budget = 12                 # budget de l'exécution, de 1 à 65535 ; 1000 s'il est omis
mem = { 3 = 42 }            # mémoire initiale creuse : cellule (0 à 255) → valeur ; les autres valent 0
answers = [{ sense = 0 }]   # réponses aux rappels, dans l'ordre où la machine les pose
```

| Champ | Sens |
|---|---|
| `description` | ce que le cas éprouve, en une phrase |
| `category` | la catégorie, la même que dans le nom du répertoire |
| `clause` | la clause de `SPEC-PBC-v1.md` éprouvée — à lire avant de déboguer |
| `[[exec]]` | une exécution ; un cas en compte une ou plusieurs, chacune indépendante des autres (K.9) |
| `budget` | le `budget` de l'`EXEC` (K.3) |
| `mem` | la `mem` de l'`EXEC`, écrite creuse : à rendre dense sur 256 cellules (B.1) |
| `answers` | la réponse à chaque rappel, dans l'ordre |
| `load_error` | présent seulement pour un **refus de chargement** : la faute de B.5 que le chargeur doit lever (A.2). Le cas n'a alors aucune exécution |

Une réponse porte exactement **une** de ces clés :

| Clé | Rappel posé par la machine | Réponse à lui rendre (K.3) |
|---|---|---|
| `sense = v` | `SENSE` | `SENSED` portant la valeur `v` |
| `sense_fault = "BadSensor"` | `SENSE` | `SENSED` portant cette faute, que la machine lève |
| `act = true` / `act = false` | `ACT` | `ACTED` portant cette issue |
| `rand = v` | `RAND` | `DREW` portant la valeur `v` |

Le capteur demandé, l'action et son argument ne figurent pas dans `answers` : c'est la machine qui les choisit, et
`expected.json` dit ce qu'elle doit choisir.

## `expected.json`

Absent pour un refus de chargement.

```json
{
  "spec_version": "1.9.0",
  "executions": [
    {
      "calls": [
        { "call": "SENSE", "k": 6, "value": 1 },
        { "call": "SENSE", "k": 200, "fault": "BadSensor" },
        { "call": "ACT", "kind": "TAKE", "arg": 0, "ok": true },
        { "call": "RAND", "value": 7 }
      ],
      "result": {
        "mem": [0, 0, 0, "… 256 entiers"],
        "gas_used": 14,
        "fault": null,
        "act": { "kind": "TAKE", "arg": 0, "ok": true },
        "trace": [1]
      }
    }
  ]
}
```

* `spec_version` — la version des règles (H.1). Une comparaison n'a de sens qu'à `MAJEUR.MINEUR` égaux.
* `executions` — une entrée par `[[exec]]` de `case.toml`, dans le même ordre.
* `calls` — le **transcript** : chaque rappel que la machine doit poser, dans l'ordre, avec la réponse qu'il reçoit.
  Il fait partie du résultat (K.5) : une machine qui pose une autre question, une de plus ou une de moins, diverge
  même si son résultat final coïncide.
* `result` — exactement le `RESULT` de K.3 :
  * `mem` : les 256 cellules, **telles que l'exécution les a laissées**. Sur une faute, les écritures antérieures y
    restent — la restauration de E.2 n'est pas le fait de la machine ;
  * `gas_used` : le gaz consommé, prélevé avant chaque instruction (B.2) ;
  * `fault` : une chaîne exacte de B.5, ou `null` ;
  * `act` : l'action tentée, `{kind, arg, ok}`, même échouée et même si l'exécution a fauté ensuite — ou `null` ;
  * `trace` : les valeurs passées à `TRACE`, dans l'ordre.

## Rejouer un cas

Pour un cas de refus de chargement : soumettre `program.pbc` ; il doit être refusé — sous K, un `SUBMIT` qui reçoit
`ERROR` avec le code `bad_program` (K.8).

Pour les autres, à chaque exécution :

1. charger `program.pbc` — sous K : `SUBMIT`, puis `SESSION` ;
2. lancer l'exécution avec `mem` (rendue dense) et `budget` — sous K : `EXEC` ;
3. à chaque rappel reçu, prendre l'entrée suivante de `calls`, **vérifier** que la question est la même (même `call`,
   même `k` pour `SENSE`, mêmes `kind` et `arg` pour `ACT`), et répondre ce qu'elle porte ;
4. à la fin, tous les rappels du script doivent avoir été posés ;
5. comparer, dans cet ordre, et s'arrêter à la première différence : les rappels, `gas_used`, `fault`, `act`,
   `trace`, puis `mem`.

L'ordre compte : un gaz faux explique presque toujours une mémoire fausse, l'inverse jamais. Corriger le premier
champ en erreur avant de regarder les suivants.
