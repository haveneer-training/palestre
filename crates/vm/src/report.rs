//! Le compte-rendu d'un tour : exactement le `RESULT` de l'annexe K.3, sous forme typée.

use pbc::Fault;

use crate::Memory;

/// Les quatre actions de E.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActKind {
    Move,
    Take,
    Drop,
    Eat,
}

impl ActKind {
    /// L'action désignée par l'opérande de `ACT k` : 0 → `Move`, 1 → `Take`, 2 → `Drop`, 3 → `Eat`, sinon `None`
    /// (ce qui donnera `Fault::BadAction`).
    pub fn from_code(k: u8) -> Option<ActKind> {
        match k {
            0 => Some(ActKind::Move),
            1 => Some(ActKind::Take),
            2 => Some(ActKind::Drop),
            3 => Some(ActKind::Eat),
            _ => None,
        }
    }

    /// Le nom qui circule sur le fil (K.3).
    pub fn name(self) -> &'static str {
        match self {
            ActKind::Move => "MOVE",
            ActKind::Take => "TAKE",
            ActKind::Drop => "DROP",
            ActKind::Eat => "EAT",
        }
    }

    /// Inverse de `name`, sensible à la casse.
    pub fn from_name(name: &str) -> Option<ActKind> {
        [ActKind::Move, ActKind::Take, ActKind::Drop, ActKind::Eat]
            .into_iter()
            .find(|kind| kind.name() == name)
    }
}

/// Une action et son argument dépilé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub kind: ActKind,
    pub arg: i64,
}

/// L'action tentée pendant le tour et son issue. Elle reste consignée même si le tour faute ensuite (E.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActRecord {
    pub action: Action,
    pub ok: bool,
}

/// Ce qu'un tour rend : la mémoire telle que l'exécution l'a laissée (même après une faute : restaurer est le travail
/// de l'arène, pas de la machine), le gaz consommé, la faute éventuelle, l'action tentée et la trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub mem: Memory,
    pub gas_used: u16,
    pub fault: Option<Fault>,
    pub act: Option<ActRecord>,
    pub trace: Vec<i64>,
}
