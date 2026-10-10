//! Reversible presentation-only workspace presets.
//!
//! Inspired by established creative-app UX: stable context menus, familiar
//! document tools, predictable workspace switching and explicit return paths.
//! No external UI source code, bundled icons or trademarks are copied.

use serde::{Deserialize, Serialize};

use crate::UiState;

/// Only workspaces with a functional native editor can be selected.
/// OrbWeaver/Scan/Terrain/etc. become variants after their editors work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Workspace {
    Drafting,
    Modeling,
}

impl Workspace {
    pub const ALL: [Self; 2] = [Self::Drafting, Self::Modeling];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Drafting => "2D Drawing",
            Self::Modeling => "3D Modeling",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Drafting => "Vector drawings, drafting, dimensions and sheets",
            Self::Modeling => "3D modeling, meshes, surfaces and model hierarchy",
        }
    }
}

/// Read legacy UI settings without requiring migration of existing profiles.
pub fn active(ui: &UiState) -> Workspace {
    if ui.buildercraft_workspace && ui.view3d {
        Workspace::Modeling
    } else {
        Workspace::Drafting
    }
}

/// One presentation-only state change; does not touch document, selection,
/// geometry, parameters, undo, scene graph or command-line history.
pub fn activate(ui: &mut UiState, target: Workspace) -> bool {
    let current = active(ui);
    if current == target {
        return false;
    }
    ui.previous_workspace = Some(current);
    match target {
        Workspace::Drafting => {
            ui.buildercraft_workspace = false;
            ui.view3d = false;
            ui.toolset_tab = "Drafting".into();
        }
        Workspace::Modeling => {
            ui.buildercraft_workspace = true;
            ui.view3d = true;
            ui.toolset_tab = "Modeling".into();
        }
    }
    true
}

/// Explicit return path. If the old profile contains a redundant return
/// target, clear that target without surprising navigation.
pub fn back(ui: &mut UiState) -> bool {
    let Some(target) = ui.previous_workspace else {
        return false;
    };
    if target == active(ui) {
        ui.previous_workspace = None;
        return false;
    }
    activate(ui, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CadApp, Services};

    #[test]
    fn switches_modes_without_altering_pinned_ui_controls() {
        let mut ui = UiState::default();
        ui.show_command_line = false;
        ui.show_palettes = false;
        ui.show_toolsets = false;
        ui.history_lines = 9;
        let old_center = ui.center3d;
        assert_eq!(active(&ui), Workspace::Modeling);
        assert!(activate(&mut ui, Workspace::Drafting));
        assert_eq!(active(&ui), Workspace::Drafting);
        assert_eq!(ui.previous_workspace, Some(Workspace::Modeling));
        assert_eq!(ui.toolset_tab, "Drafting");
        assert!(!ui.show_command_line);
        assert!(!ui.show_palettes);
        assert!(!ui.show_toolsets);
        assert_eq!(ui.history_lines, 9);
        assert_eq!(ui.center3d, old_center);
        assert!(!activate(&mut ui, Workspace::Drafting));
        assert_eq!(ui.previous_workspace, Some(Workspace::Modeling));
        assert!(back(&mut ui));
        assert_eq!(active(&ui), Workspace::Modeling);
        assert_eq!(ui.previous_workspace, Some(Workspace::Drafting));
    }

    #[test]
    fn preserves_old_profiles_and_roundtrips_new_return_state() {
        let mut old: UiState = serde_json::from_str(
            r#"{"buildercraftWorkspace":false,"view3d":false,"showCommandLine":true}"#,
        ).unwrap();
        assert_eq!(active(&old), Workspace::Drafting);
        assert_eq!(old.previous_workspace, None);
        assert!(activate(&mut old, Workspace::Modeling));
        let encoded = serde_json::to_string(&old).unwrap();
        let restored: UiState = serde_json::from_str(&encoded).unwrap();
        assert_eq!(active(&restored), Workspace::Modeling);
        assert_eq!(restored.previous_workspace, Some(Workspace::Drafting));
    }

    #[test]
    fn active_session_revision_and_selection_do_not_change_on_switch() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), Services::default());
        let before = app.session.state().unwrap();
        let selection = app.session.selection().to_vec();
        let history = app.cmd.history.clone();
        assert!(activate(&mut app.ui, Workspace::Drafting));
        assert!(activate(&mut app.ui, Workspace::Modeling));
        let after = app.session.state().unwrap();
        assert_eq!(before.uid, after.uid);
        assert_eq!(before.revision, after.revision);
        assert_eq!(app.session.selection(), selection.as_slice());
        assert_eq!(app.cmd.history, history);
    }
}
