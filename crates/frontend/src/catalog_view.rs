//! Catalog-owned modal content; the shell owns dialog framing and focus.
use crate::App;

pub(crate) fn catalog(app: &App) -> String {
    app.catalog.panel()
}

pub(crate) fn reinforcements(app: &App) -> String {
    app.inventory.panel()
}
