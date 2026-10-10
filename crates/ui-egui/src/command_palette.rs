//! Searchable native command launcher backed by the existing command registry.
//!
//! No new geometry implementation: commands run through CadApp::start just as
//! menus, the command line and toolsets do. The search index is transient UI.

use egui::{Key, vec2};

use crate::CadApp;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteHit {
    pub id: String,
    pub label: String,
    pub rank: u8,
}

fn relevance(id: &str, label: &str, query: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(6);
    }
    let id = id.to_ascii_lowercase();
    let label = label.to_ascii_lowercase();
    if id == query {
        Some(0)
    } else if id.starts_with(query) {
        Some(1)
    } else if label.starts_with(query) {
        Some(2)
    } else if id.contains(query) {
        Some(3)
    } else if label.contains(query) {
        Some(4)
    } else {
        None
    }
}

/// Stable, bounded search across engine and UI commands. An alias match is
/// weaker than a command-ID match, so predictable exact commands come first.
pub fn hits(query: &str, limit: usize) -> Vec<PaletteHit> {
    let query = query.trim().chars().take(80).collect::<String>().to_ascii_lowercase();
    let mut results = Vec::new();
    for command in cadcraft_engine::command_specs() {
        let score = relevance(command.id, command.label, &query)
            .or_else(|| command.aliases.iter().any(|a| a.to_ascii_lowercase().contains(&query)).then_some(5));
        if let Some(rank) = score {
            results.push(PaletteHit { id: command.id.into(), label: command.label.into(), rank });
        }
    }
    for &(id, label, _, _) in crate::menus::UI_COMMANDS {
        if matches!(id, "ui.noop" | "ui.quit" | "ui.dialog.palette") || label.is_empty() {
            continue;
        }
        if let Some(rank) = relevance(id, label, &query) {
            results.push(PaletteHit { id: id.into(), label: label.into(), rank });
        }
    }
    results.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.id.cmp(&b.id)));
    results.truncate(limit.min(40));
    results
}

/// Popup invoked by Window > Command Palette, or Cmd/Ctrl+K when the viewport
/// owns keyboard focus. Both pointer and keyboard submission use CadApp::start.
pub fn show(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) {
    let query_id = egui::Id::new("worldwright_command_palette_query");
    let selection_id = egui::Id::new("worldwright_command_palette_selection");
    let mut query = ctx.data_mut(|d| d.get_temp::<String>(query_id)).unwrap_or_default();
    let mut selected = ctx.data_mut(|d| d.get_temp::<usize>(selection_id)).unwrap_or(0);
    let mut chosen = None;
    let mut dismissed = false;
    egui::Window::new("Worldwright Command Palette").open(open).default_size(vec2(530.0, 370.0)).collapsible(false).show(ctx, |ui| {
        ui.label("Find a command by name, ID or alias. Enter runs the highlighted command.");
        let previous = query.clone();
        let response = ui.add(
            egui::TextEdit::singleline(&mut query)
                .id(query_id.with("input"))
                .hint_text("Line, Move, 3D Modeling, Save…")
                .desired_width(f32::INFINITY),
        );
        if !response.has_focus() {
            response.request_focus();
        }
        if previous != query {
            selected = 0;
        }
        let options = hits(&query, 40);
        if !options.is_empty() {
            selected = selected.min(options.len() - 1);
        }
        let down = ui.input(|i| i.key_pressed(Key::ArrowDown));
        let up = ui.input(|i| i.key_pressed(Key::ArrowUp));
        if down && !options.is_empty() {
            selected = (selected + 1).min(options.len() - 1);
        }
        if up {
            selected = selected.saturating_sub(1);
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            dismissed = true;
        }
        if ui.input(|i| i.key_pressed(Key::Enter)) && !query.trim().is_empty() {
            chosen = options.get(selected).map(|x| x.id.clone());
        }
        ui.separator();
        egui::ScrollArea::vertical().max_height(290.0).show(ui, |ui| {
            if options.is_empty() {
                ui.weak("No matching registered command.");
            }
            for (index, option) in options.iter().enumerate() {
                let text = format!("{}    {}", option.label, option.id);
                if ui.selectable_label(index == selected, text).clicked() {
                    chosen = Some(option.id.clone());
                }
            }
        });
    });
    ctx.data_mut(|d| {
        d.insert_temp(query_id, query);
        d.insert_temp(selection_id, selected);
    });
    if dismissed || chosen.is_some() {
        *open = false;
    }
    if let Some(command_id) = chosen {
        ctx.data_mut(|d| {
            d.remove::<String>(query_id);
            d.remove::<usize>(selection_id);
        });
        app.start(&command_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_prefix_matches_are_ordered() {
        let options = hits("line", 40);
        assert!(!options.is_empty());
        assert_eq!(options[0].id, "line");
        assert_eq!(options[0].rank, 0);
    }

    #[test]
    fn includes_workspace_commands_without_copying_the_engine() {
        let options = hits("ui.workspace.modeling", 40);
        assert_eq!(options.first().map(|x| x.id.as_str()), Some("ui.workspace.modeling"));
    }

    #[test]
    fn query_is_bounded_and_missing_commands_do_not_execute() {
        assert!(hits("command-name-definitely-not-registered-01234567890", 40).is_empty());
        assert!(hits("line", 0).is_empty());
        assert!(hits(" ", 1000).len() <= 40);
    }

    #[test]
    fn excludes_quit_and_self_invocation_from_search() {
        assert!(!hits("ui.quit", 40).iter().any(|x| x.id == "ui.quit"));
        assert!(!hits("ui.dialog.palette", 40).iter().any(|x| x.id == "ui.dialog.palette"));
    }
}
