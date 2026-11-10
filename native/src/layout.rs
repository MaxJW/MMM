//! Which widget ids show in which area. The Slint side (`areas` in ui/app.slint) is one
//! list of ids per area; each entry renders through `WidgetSlot` in ui/widgets.slint.
//!
//! Only implemented widgets are placed; a widget can also take itself out of its area
//! (`set_visible(ui, ID, false)`) while it has nothing to show, so the area's gap
//! closes up as it does in the web app when a component renders nothing.

use std::cell::RefCell;
use std::collections::HashSet;

use slint::{Model, ModelRc, SharedString, VecModel};

use crate::config::{Area, Config, Placement};
use crate::AppWindow;

#[derive(Default)]
struct State {
    placements: Vec<Placement>,
    hidden: HashSet<String>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

pub fn init(ui: &AppWindow, config: &Config) {
    let mut placements = Vec::new();
    for p in &config.placements {
        if crate::widgets::is_known(&p.id) {
            placements.push(p.clone());
        } else {
            eprintln!("layout: no native widget `{}` yet; skipped", p.id);
        }
    }
    let rows: Vec<ModelRc<SharedString>> = Area::ALL
        .iter()
        .map(|_| ModelRc::new(VecModel::<SharedString>::default()))
        .collect();
    ui.set_areas(ModelRc::new(VecModel::from(rows)));
    STATE.with_borrow_mut(|s| s.placements = placements);
    apply(ui);
}

/// Show or hide every placement of widget `id`. Cheap when nothing changes.
pub fn set_visible(ui: &AppWindow, id: &str, visible: bool) {
    let changed = STATE.with_borrow_mut(|s| {
        if visible {
            s.hidden.remove(id)
        } else {
            s.hidden.insert(id.into())
        }
    });
    if changed {
        apply(ui);
    }
}

/// Push each area's id list to Slint, touching only areas whose list changed.
fn apply(ui: &AppWindow) {
    let areas = ui.get_areas();
    STATE.with_borrow(|s| {
        for (i, area) in Area::ALL.iter().enumerate() {
            let ids: Vec<SharedString> = s
                .placements
                .iter()
                .filter(|p| p.area == *area && !s.hidden.contains(&p.id))
                .map(|p| p.id.as_str().into())
                .collect();
            let current = areas.row_data(i).unwrap_or_default();
            if !current.iter().eq(ids.iter().cloned()) {
                areas.set_row_data(i, ModelRc::new(VecModel::from(ids)));
            }
        }
    });
}
