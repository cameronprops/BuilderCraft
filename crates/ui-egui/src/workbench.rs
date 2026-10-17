//! Versioned user workspace preferences, separate from geometry and document identity.
use crate::CadApp;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Two modeling perspectives over the same document, not separate kernels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelingMode {
    #[default]
    WorldBuilder,
    SolidCraft,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Layout {
    pub version: u32,
    pub left_width: f32,
    pub right_width: f32,
    pub command_height: f32,
    pub tools: bool,
    pub inspector: bool,
    pub toolbar: bool,
    pub command_line: bool,
    pub view3d: bool,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            version: 1,
            left_width: 220.,
            right_width: 300.,
            command_height: 78.,
            tools: true,
            inspector: true,
            toolbar: true,
            command_line: true,
            view3d: true,
        }
    }
}
impl Layout {
    fn capture(app: &CadApp) -> Self {
        Self {
            tools: app.ui.show_toolsets,
            inspector: app.ui.show_palettes,
            toolbar: app.ui.show_toolbar,
            command_line: app.ui.show_command_line,
            view3d: app.ui.view3d,
            ..app.ui.layout.clone()
        }
    }
    fn apply(&self, app: &mut CadApp) -> Result<(), String> {
        if self.version != 1 || !self.left_width.is_finite() || !self.right_width.is_finite() || !self.command_height.is_finite() {
            return Err("Unsupported or invalid workspace profile; existing workspace retained".into());
        }
        let mut layout = self.clone();
        layout.left_width = layout.left_width.clamp(190., 360.);
        layout.right_width = layout.right_width.clamp(240., 480.);
        layout.command_height = layout.command_height.clamp(60., 180.);
        app.ui.show_toolsets = layout.tools;
        app.ui.show_palettes = layout.inspector;
        app.ui.show_toolbar = layout.toolbar;
        app.ui.show_command_line = layout.command_line;
        // Finish or cancel drafting before changing the construction context.
        if app.ui.view3d != layout.view3d {
            app.ui.point_input = Default::default();
            app.ui.gizmo = Default::default();
        }
        app.ui.view3d = layout.view3d;
        app.ui.toolset_tab = if layout.view3d { "Modeling" } else { "Drafting" }.into();
        app.ui.buildercraft_workspace = true;
        app.ui.layout = layout;
        app.ui.layout_dirty = true;
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
struct Preferences {
    active: Layout,
    saved: Option<Layout>,
    #[serde(default)]
    shelf: Vec<String>,
    #[serde(default)]
    modeling_mode: ModelingMode,
}
/// Only user workspace fields are persisted. Selection, camera, dialogs and geometry are excluded.
pub fn encode(app: &CadApp) -> Result<String, String> {
    validate_shelf(&app.ui.tool_shelf)?;
    serde_json::to_string(&Preferences {
        active: Layout::capture(app),
        saved: app.ui.saved_layout.clone(),
        shelf: app.ui.tool_shelf.clone(),
        modeling_mode: app.ui.modeling_mode,
    })
    .map_err(|e| e.to_string())
}
pub fn restore(app: &mut CadApp, text: &str) -> Result<(), String> {
    if text.len() > 16_384 {
        return Err("Workspace profile exceeds size limit".into());
    }
    let profile: Preferences = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if let Some(saved) = &profile.saved
        && (saved.version != 1 || !saved.left_width.is_finite() || !saved.right_width.is_finite() || !saved.command_height.is_finite())
    {
        return Err("Invalid saved workspace profile".into());
    }
    validate_shelf(&profile.shelf)?;
    profile.active.apply(app)?;
    app.ui.tool_shelf = profile.shelf;
    app.ui.saved_layout = profile.saved;
    app.ui.modeling_mode = profile.modeling_mode;
    Ok(())
}
pub fn command(app: &mut CadApp, id: &str) -> Result<Value, String> {
    match id {
        "ui.workspace.save" => {
            app.ui.saved_layout = Some(Layout::capture(app));
            app.set_status("Custom layout saved");
        }
        "ui.workspace.restore" => {
            let layout = app.ui.saved_layout.clone().ok_or("No custom layout saved yet")?;
            layout.apply(app)?;
        }
        "ui.workspace.modeling" => {
            Layout::default().apply(app)?;
            app.ui.modeling_mode = ModelingMode::WorldBuilder;
        }
        "ui.workspace.worldbuilder" | "ui.workspace.solidcraft" => {
            // A workspace switch changes only UI state. CAD identities,
            // exact BReps, selection, feature timelines, undo and revision
            // are deliberately untouched.
            Layout { view3d: true, ..Layout::capture(app) }.apply(app)?;
            app.ui.modeling_mode = if id == "ui.workspace.solidcraft" { ModelingMode::SolidCraft } else { ModelingMode::WorldBuilder };
        }
        "ui.workspace.drafting" => Layout { view3d: false, ..Default::default() }.apply(app)?,
        "ui.workspace.focus" => Layout { tools: false, inspector: false, toolbar: false, ..Layout::capture(app) }.apply(app)?,
        _ => return Err(format!("Unknown workspace command: {id}")),
    }
    Ok(json!({"workspaceVersion":1,"modelingMode":app.ui.modeling_mode}))
}
#[derive(Clone, Debug, Default)]
pub struct Search {
    pub open: bool,
    pub focus: bool,
    query: String,
    selected: usize,
    accept: bool,
    cancel: bool,
    down: bool,
    up: bool,
}
struct Match {
    id: &'static str,
    label: &'static str,
    enabled: bool,
    reason: String,
}
fn matches(app: &CadApp, query: &str) -> Vec<Match> {
    let terms: Vec<_> = query.split_whitespace().take(16).map(str::to_lowercase).collect();
    let mut rows = Vec::new();
    for c in cadcraft_engine::command_specs() {
        let haystack = format!("{} {} {} {}", c.id, c.label, c.menu.join(" "), c.aliases.join(" ")).to_lowercase();
        if terms.iter().all(|term| haystack.contains(term)) {
            let availability = (c.enabled)(&app.session);
            rows.push(Match {
                id: c.id,
                label: c.label,
                enabled: availability.is_ok(),
                reason: availability.err().map(|e| e.to_string()).unwrap_or_default(),
            });
        }
    }
    for (id, label, path, _) in crate::menus::UI_COMMANDS {
        if label.is_empty() || *id == "ui.command.search" {
            continue;
        }
        let haystack = format!("{id} {label} {}", path.join(" ")).to_lowercase();
        if terms.iter().all(|term| haystack.contains(term)) {
            let enabled = *id != "ui.workspace.restore" || app.ui.saved_layout.is_some();
            rows.push(Match { id, label, enabled, reason: if enabled { String::new() } else { "Save a custom layout first".into() } });
        }
    }
    rows.sort_by_key(|row| (!row.enabled, row.label, row.id));
    rows.truncate(80);
    rows
}
/// Consume modal navigation before any viewport, toolbar or command-line widget sees it.
pub fn shortcuts(app: &mut CadApp, ctx: &egui::Context) {
    if !app.ui.command_search.open {
        return;
    }
    ctx.input_mut(|i| {
        let search = &mut app.ui.command_search;
        search.accept |= i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
        search.cancel |= i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        search.down |= i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
        search.up |= i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
    });
}
pub fn search(app: &mut CadApp, ctx: &egui::Context) {
    if !app.ui.command_search.open {
        return;
    }
    let mut chosen = None;
    let mut close = std::mem::take(&mut app.ui.command_search.cancel);
    let accept = std::mem::take(&mut app.ui.command_search.accept);
    let down = std::mem::take(&mut app.ui.command_search.down);
    let up = std::mem::take(&mut app.ui.command_search.up);
    egui::Modal::new(egui::Id::new("worldwright_command_search")).show(ctx, |ui| {
        ui.set_width(540.);
        ui.heading("Search commands");
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.ui.command_search.query)
                .hint_text("Command, tool, alias or category")
                .char_limit(256)
                .desired_width(f32::INFINITY),
        );
        if app.ui.command_search.focus {
            response.request_focus();
            app.ui.command_search.focus = false;
        }
        if response.changed() {
            app.ui.command_search.selected = 0;
        }
        let rows = matches(app, &app.ui.command_search.query);
        if down {
            app.ui.command_search.selected = (app.ui.command_search.selected + 1).min(rows.len().saturating_sub(1));
        }
        if up {
            app.ui.command_search.selected = app.ui.command_search.selected.saturating_sub(1);
        }
        if accept
            && !close
            && let Some(row) = rows.get(app.ui.command_search.selected)
            && row.enabled
        {
            chosen = Some(row.id);
        }
        ui.weak("↑ ↓ Choose   Enter Run   Esc Close");
        egui::ScrollArea::vertical().max_height(330.).show(ui, |ui| {
            if rows.is_empty() {
                ui.label("No matching commands. Try a tool name or category.");
            }
            for (index, row) in rows.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!app.ui.tool_shelf.iter().any(|id| id == row.id) && app.ui.tool_shelf.len() < 32, egui::Button::new("Pin"))
                        .clicked()
                    {
                        let _ = pin(app, row.id);
                    }
                    let response = ui
                        .add_enabled(
                            row.enabled,
                            egui::Button::selectable(index == app.ui.command_search.selected, format!("{}   {}", row.label, row.id)),
                        )
                        .on_hover_text(&row.reason);
                    if index == app.ui.command_search.selected {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if response.clicked() {
                        chosen = Some(row.id);
                    }
                });
            }
        });
        if ui.button("Close").clicked() {
            close = true;
        }
    });
    if close || chosen.is_some() {
        app.ui.command_search = Default::default();
    }
    if let Some(id) = chosen {
        app.start(id);
    }
}

fn validate_shelf(ids: &[String]) -> Result<(), String> {
    if ids.len() > 32 || ids.iter().any(|id| id.is_empty() || id.len() > 128 || !id.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)))
    {
        return Err("Invalid tool shelf or shelf budget exceeded".into());
    }
    let mut unique = std::collections::BTreeSet::new();
    if ids.iter().any(|id| !unique.insert(id)) {
        return Err("Duplicate shelf command".into());
    }
    Ok(())
}
pub fn pin(app: &mut CadApp, id: &str) -> Result<(), String> {
    let mut ids = app.ui.tool_shelf.clone();
    ids.push(id.into());
    validate_shelf(&ids)?;
    app.ui.tool_shelf = ids;
    app.ui.layout_dirty = true;
    Ok(())
}
fn edit_shelf(app: &mut CadApp, index: usize, direction: i8) -> Result<(), String> {
    let len = app.ui.tool_shelf.len();
    if index >= len {
        return Err("Shelf entry no longer exists".into());
    }
    match direction {
        -1 if index > 0 => app.ui.tool_shelf.swap(index, index - 1),
        1 if index + 1 < len => app.ui.tool_shelf.swap(index, index + 1),
        0 => {
            app.ui.tool_shelf.remove(index);
        }
        _ => return Err("Invalid shelf move".into()),
    }
    app.ui.layout_dirty = true;
    Ok(())
}
pub fn shelf(app: &mut CadApp, ui: &mut egui::Ui) {
    if !app.ui.show_toolbar || app.ui.tool_shelf.is_empty() {
        return;
    }
    let mut chosen = None;
    let mut edit = None;
    egui::Panel::top("worldwright_tool_shelf").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.weak("My tools");
            ui.menu_button("Edit", |ui| {
                for (i, id) in app.ui.tool_shelf.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(id);
                        if ui.add_enabled(i > 0, egui::Button::new("↑")).clicked() {
                            edit = Some((i, -1));
                        }
                        if ui.add_enabled(i + 1 < app.ui.tool_shelf.len(), egui::Button::new("↓")).clicked() {
                            edit = Some((i, 1));
                        }
                        if ui.button("Remove").clicked() {
                            edit = Some((i, 0));
                        }
                    });
                }
            });
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for id in &app.ui.tool_shelf {
                        let row = matches(app, id).into_iter().find(|r| r.id == id);
                        let (label, enabled, reason) = row.as_ref().map(|r| (r.label, r.enabled, r.reason.as_str())).unwrap_or((
                            id.as_str(),
                            false,
                            "Command unavailable; remove it using Edit",
                        ));
                        if ui.add_enabled(enabled, egui::Button::new(label)).on_hover_text(reason).clicked() {
                            chosen = Some(id.clone());
                        }
                    }
                });
            });
        });
    });
    if let Some((i, direction)) = edit {
        let _ = edit_shelf(app, i, direction);
    }
    if let Some(id) = chosen {
        app.start(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> CadApp {
        CadApp::new(cadcraft_engine::Session::new(), crate::Services::default())
    }
    #[test]
    fn worldbuilder_and_solidcraft_share_the_same_undoable_document() {
        let mut app = app();
        app.run("line", json!({"points":[[0,0],[4,4]]})).unwrap();
        app.run("worldwright.history.create", json!({"scope":{"kind":"document"}})).unwrap();
        let before = app.session.doc().unwrap().clone();
        let revision = app.session.state().unwrap().revision;
        let undo_count = app.session.state().unwrap().undo.len();
        let selection = app.session.selection();

        command(&mut app, "ui.workspace.solidcraft").unwrap();
        assert_eq!(app.ui.modeling_mode, ModelingMode::SolidCraft);
        assert!(app.ui.view3d);
        assert_eq!(app.session.doc().unwrap(), &before);
        assert_eq!(app.session.state().unwrap().revision, revision);
        assert_eq!(app.session.state().unwrap().undo.len(), undo_count);
        assert_eq!(app.session.selection(), selection);

        command(&mut app, "ui.workspace.drafting").unwrap();
        assert!(!app.ui.view3d);
        assert_eq!(app.ui.modeling_mode, ModelingMode::SolidCraft);
        command(&mut app, "ui.workspace.worldbuilder").unwrap();
        assert_eq!(app.ui.modeling_mode, ModelingMode::WorldBuilder);
        assert_eq!(app.session.doc().unwrap(), &before);
        assert_eq!(app.session.state().unwrap().revision, revision);
        assert_eq!(app.session.state().unwrap().undo.len(), undo_count);
    }

    #[test]
    fn modeling_mode_persists_in_preferences_and_legacy_defaults_to_worldbuilder() {
        let mut app = app();
        command(&mut app, "ui.workspace.solidcraft").unwrap();
        let saved = encode(&app).unwrap();
        let mut reopened = self::app();
        restore(&mut reopened, &saved).unwrap();
        assert_eq!(reopened.ui.modeling_mode, ModelingMode::SolidCraft);
        let mut legacy: serde_json::Value = serde_json::from_str(&saved).unwrap();
        legacy.as_object_mut().unwrap().remove("modeling_mode");
        restore(&mut reopened, &legacy.to_string()).unwrap();
        assert_eq!(reopened.ui.modeling_mode, ModelingMode::WorldBuilder);
        // Invalid profile data must not partially replace a valid preference.
        let mut invalid: serde_json::Value = serde_json::from_str(&saved).unwrap();
        invalid["active"]["version"] = json!(999);
        assert!(restore(&mut reopened, &invalid.to_string()).is_err());
        assert_eq!(reopened.ui.modeling_mode, ModelingMode::WorldBuilder);
    }

    #[test]
    fn layouts_preserve_document_and_roundtrip_custom_preferences() {
        let mut app = app();
        app.run("line", json!({"points":[[0,0],[4,4]]})).unwrap();
        let before = format!("{:?}", app.session.state().unwrap());
        app.ui.layout.left_width = 270.;
        command(&mut app, "ui.workspace.save").unwrap();
        command(&mut app, "ui.workspace.drafting").unwrap();
        assert!(!app.ui.view3d);
        command(&mut app, "ui.workspace.focus").unwrap();
        assert!(!app.ui.show_palettes);
        command(&mut app, "ui.workspace.restore").unwrap();
        assert_eq!(app.ui.layout.left_width, 270.);
        assert!(app.ui.view3d);
        assert_eq!(before, format!("{:?}", app.session.state().unwrap()));
        let text = encode(&app).unwrap();
        let mut reopened = self::app();
        restore(&mut reopened, &text).unwrap();
        assert_eq!(reopened.ui.layout.left_width, 270.);
        assert!(reopened.ui.saved_layout.is_some());
        let bad = text.replace("\"version\":1", "\"version\":99");
        assert!(restore(&mut reopened, &bad).is_err());
        assert_eq!(reopened.ui.layout.left_width, 270.);
        assert!(restore(&mut reopened, &"x".repeat(16_385)).is_err());
    }
    fn frame(app: &mut CadApp, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440., 900.))), events, ..Default::default() },
            |ui| {
                app.logic(ui.ctx());
                app.ui(ui);
            },
        );
        if std::env::var_os("WORLDWRIGHT_WORKBENCH_CAPTURE").is_some() {
            for delta in output.textures_delta.set.values().flatten() {
                let egui::ImageData::Color(texture) = &delta.image;
                image::RgbaImage::from_raw(
                    texture.size[0] as u32,
                    texture.size[1] as u32,
                    texture.pixels.iter().flat_map(|p| p.to_array()).collect(),
                )
                .unwrap()
                .save("/tmp/worldwright-workbench-atlas.png")
                .unwrap();
            }
        }
        output.textures_delta.clear();
        output
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }
    }
    #[test]
    fn keyboard_search_launch_close_and_panel_restore() {
        let mut app = app();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        app.ui.layout.left_width = 280.;
        app.ui.layout.right_width = 360.;
        app.ui.layout.command_height = 120.;
        app.start("ui.workspace.save");
        app.start("ui.workspace.modeling");
        frame(&mut app, &ctx, vec![]);
        app.start("ui.workspace.restore");
        frame(&mut app, &ctx, vec![]);
        assert!((app.ui.layout.left_width - 280.).abs() < 1.);
        assert!((app.ui.layout.command_height - 120.).abs() < 1., "actual command height {}", app.ui.layout.command_height);
        assert!((app.ui.layout.right_width - 360.).abs() < 1., "actual width {}", app.ui.layout.right_width);
        let document_before_search = format!("{:?}", app.session.state().unwrap());
        app.start("ui.command.search");
        app.ui.command_search.query = "ui.buildercraft.drawcurve".into();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert!(!app.ui.command_search.open);
        assert!(crate::point_input::active(&app));
        app.start("ui.command.search");
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert!(!app.ui.command_search.open);
        assert!(crate::point_input::active(&app));
        assert_eq!(document_before_search, format!("{:?}", app.session.state().unwrap()));
        if std::env::var_os("WORLDWRIGHT_WORKBENCH_CAPTURE").is_some() {
            app.ui.point_input = Default::default();
            pin(&mut app, "ui.buildercraft.drawcurve").unwrap();
            pin(&mut app, "ui.workspace.focus").unwrap();
            app.start("ui.buildercraft.curve");
            let capture = egui::Context::default();
            for _ in 0..2 {
                let output = frame(&mut app, &capture, vec![]);
                let meshes: Vec<_> = capture.tessellate(output.shapes, 1.).into_iter().filter_map(|p| {
                    let egui::epaint::Primitive::Mesh(m) = p.primitive else { return None; };
                    Some(json!({"clip":[p.clip_rect.min.x,p.clip_rect.min.y,p.clip_rect.max.x,p.clip_rect.max.y],"indices":m.indices,"vertices":m.vertices.iter().map(|v|json!([v.pos.x,v.pos.y,v.uv.x,v.uv.y,v.color.to_array()])).collect::<Vec<_>>()}))
                }).collect();
                std::fs::write("/tmp/worldwright-workbench-meshes.json", serde_json::to_vec(&meshes).unwrap()).unwrap();
            }
        }
    }
    #[test]
    fn search_uses_registered_commands_and_context_availability() {
        let mut app = app();
        assert!(matches(&app, "draw control").iter().any(|r| r.id == "ui.buildercraft.drawcurve"));
        assert!(matches(&app, "restore custom").iter().any(|r| !r.enabled));
        app.start("ui.workspace.save");
        assert!(matches(&app, "restore custom").iter().any(|r| r.enabled));
        assert!(matches(&app, "never_a_real_command").is_empty());
    }

    #[test]
    fn disabled_search_result_cannot_run_or_replace_workspace() {
        let mut app = app();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let before = encode(&app).unwrap();
        app.start("ui.command.search");
        app.ui.command_search.query = "ui.workspace.restore".into();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert!(app.ui.command_search.open);
        assert_eq!(before, encode(&app).unwrap());
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert!(!app.ui.command_search.open);
    }

    #[test]
    fn search_navigation_and_shortcuts_leave_document_unchanged() {
        let mut app = app();
        app.run("line", json!({"points":[[0,0],[4,4]]})).unwrap();
        let before = format!("{:?}", app.session.state().unwrap());
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        app.start("ui.command.search");
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowDown)]);
        assert_eq!(app.ui.command_search.selected, 1);
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowUp)]);
        assert_eq!(app.ui.command_search.selected, 0);
        let mut undo = key(egui::Key::Z);
        if let egui::Event::Key { modifiers, .. } = &mut undo {
            *modifiers = egui::Modifiers::COMMAND;
        }
        frame(&mut app, &ctx, vec![undo]);
        app.ui.command_search.query = "never_a_real_command".into();
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert!(app.ui.command_search.open);
        assert_eq!(before, format!("{:?}", app.session.state().unwrap()));
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert!(!app.ui.command_search.open);
    }
}

#[cfg(test)]
mod shelf_tests {
    use super::*;
    #[test]
    fn shelf_roundtrip_legacy_and_atomic_rejection() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let document_before = format!("{:?}", app.session.state().unwrap());
        pin(&mut app, "ui.buildercraft.drawcurve").unwrap();
        assert_eq!(document_before, format!("{:?}", app.session.state().unwrap()));
        let before = encode(&app).unwrap();
        assert!(pin(&mut app, "ui.buildercraft.drawcurve").is_err());
        assert_eq!(before, encode(&app).unwrap());
        let mut other = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        restore(&mut other, &before).unwrap();
        assert_eq!(app.ui.tool_shelf, other.ui.tool_shelf);
        let mut legacy: Value = serde_json::from_str(&before).unwrap();
        legacy.as_object_mut().unwrap().remove("shelf");
        restore(&mut other, &legacy.to_string()).unwrap();
        assert!(other.ui.tool_shelf.is_empty());
        let mut bad: Value = serde_json::from_str(&before).unwrap();
        bad["shelf"] = json!(["bad id"]);
        assert!(restore(&mut app, &bad.to_string()).is_err());
        assert_eq!(before, encode(&app).unwrap());
    }
    #[test]
    fn shelf_order_limits_and_missing_commands() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        pin(&mut app, "ui.workspace.focus").unwrap();
        pin(&mut app, "missing.future.command").unwrap();
        edit_shelf(&mut app, 1, -1).unwrap();
        assert_eq!(app.ui.tool_shelf[0], "missing.future.command");
        edit_shelf(&mut app, 0, 1).unwrap();
        assert_eq!(app.ui.tool_shelf[0], "ui.workspace.focus");
        let before = encode(&app).unwrap();
        assert!(edit_shelf(&mut app, usize::MAX, 1).is_err());
        assert!(edit_shelf(&mut app, 0, -1).is_err());
        assert_eq!(encode(&app).unwrap(), before);
        edit_shelf(&mut app, 1, 0).unwrap();
        for i in 1..32 {
            pin(&mut app, &format!("fixture.{i}")).unwrap();
        }
        let before = encode(&app).unwrap();
        assert!(pin(&mut app, "fixture.extra").is_err());
        assert_eq!(before, encode(&app).unwrap());
    }
    fn text_center(shapes: &[egui::epaint::ClippedShape], label: &str) -> Option<egui::Pos2> {
        fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => Some(text.pos + text.galley.size() * 0.5),
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
                _ => None,
            }
        }
        shapes.iter().find_map(|shape| find(&shape.shape, label))
    }
    fn pointer_frame(app: &mut CadApp, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut out = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440., 900.))), events, ..Default::default() },
            |ui| {
                app.logic(ui.ctx());
                app.ui(ui);
            },
        );
        out.textures_delta.clear();
        out
    }
    fn click(app: &mut CadApp, ctx: &egui::Context, pos: egui::Pos2) {
        pointer_frame(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() },
            ],
        );
        pointer_frame(
            app,
            ctx,
            vec![egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }],
        );
    }
    #[test]
    fn pointer_pin_does_not_run_and_shelf_launch_reuses_handler() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let ctx = egui::Context::default();
        pointer_frame(&mut app, &ctx, vec![]);
        let before = format!("{:?}", app.session.state().unwrap());
        app.start("ui.command.search");
        app.ui.command_search.query = "ui.buildercraft.drawcurve".into();
        pointer_frame(&mut app, &ctx, vec![]);
        let output = pointer_frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, text_center(&output.shapes, "Pin").unwrap());
        assert_eq!(app.ui.tool_shelf, ["ui.buildercraft.drawcurve"]);
        assert!(!crate::point_input::active(&app));
        assert_eq!(before, format!("{:?}", app.session.state().unwrap()));
        app.ui.command_search = Default::default();
        pointer_frame(&mut app, &ctx, vec![]);
        let output = pointer_frame(&mut app, &ctx, vec![]);
        let label = matches(&app, "ui.buildercraft.drawcurve").into_iter().find(|r| r.id == "ui.buildercraft.drawcurve").unwrap().label;
        click(&mut app, &ctx, text_center(&output.shapes, label).unwrap());
        assert!(crate::point_input::active(&app));
        assert_eq!(before, format!("{:?}", app.session.state().unwrap()));
    }
}
