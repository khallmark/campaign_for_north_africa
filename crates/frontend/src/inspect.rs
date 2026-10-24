//! Formation inspection labels. Visibility policy remains unchanged by the split.
use crate::App;
use cna_sim::Unit;

impl App {
    pub(crate) fn selected(&self) -> Option<&Unit> {
        self.game
            .units
            .iter()
            .find(|u| u.id == self.selected && u.hp > 0)
    }
}

pub(crate) fn side(unit: &Unit) -> &'static str {
    if unit.side == "allied" {
        "Commonwealth"
    } else {
        "Axis forces"
    }
}
