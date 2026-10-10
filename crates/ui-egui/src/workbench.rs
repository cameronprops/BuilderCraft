//! Versioned user workspace preferences, separate from geometry and document identity.
use crate::CadApp;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
    /// Restore the specialized metrology workspace independently of CAD.
    pub mesh_repair: bool,
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
            mesh_repair: false,
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
            mesh_repair: app.ui.mesh_repair.active,
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
        crate::mesh_repair::set_active(app, layout.mesh_repair);
        app.ui.layout = layout;
        app.ui.layout_dirty = true;
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
struct Preferences {
    active: Layout,
    saved: Option<Layout>,
}
/// Only user workspace fields are persisted. Selection, camera, dialogs and geometry are excluded.
pub fn encode(app: &CadApp) -> Result<String, String> {
    serde_json::to_string(&Preferences { active: Layout::capture(app), saved: app.ui.saved_layout.clone() }).map_err(|e| e.to_string())
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
    profile.active.apply(app)?;
    app.ui.saved_layout = profile.saved;
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
        "ui.workspace.modeling" => Layout::default().apply(app)?,
        "ui.workspace.drafting" => Layout { view3d: false, ..Default::default() }.apply(app)?,
        "ui.workspace.mesh_repair" => Layout {
            mesh_repair: true, view3d: true, tools: false, inspector: false,
            command_line: false, left_width: 230., right_width: 285.,
            ..Default::default()
        }.apply(app)?,
        "ui.workspace.focus" => Layout { tools: false, inspector: false, toolbar: false, ..Layout::capture(app) }.apply(app)?,
        _ => return Err(format!("Unknown workspace command: {id}")),
    }
    Ok(json!({"workspaceVersion":1}))
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

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> CadApp {
        CadApp::new(cadcraft_engine::Session::new(), crate::Services::default())
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
