//! The command registry. Ids are AutoCAD command names in lower case; menu paths follow
//! AutoCAD's menu bar so the catalog doubles as the parity metric.

mod annotate;
mod blocks;
mod brep3d;
mod buildercraft;
mod closest3d;
pub mod constraints;
mod draw;
mod draw2;
mod edit;
mod feature_history;
pub mod file;
mod gripcmds;
mod hatch;
mod inquiry;
mod layer;
mod layout;
mod mesh3d;
mod mesh_exchange;
mod mesh_surface;
mod modify;
mod modify2;
mod nurbs_document;
mod picking3d;
mod point3d;
mod props;
mod qselect;
mod settings;
mod snap3d;
mod table;
mod utility;
mod view;
mod worldwright_tools;

pub mod curves;
pub mod helpers;
pub mod machines;

use cadcraft_doc::Handle;
use cadcraft_geom::Vec2;
use serde::Serialize;
use serde_json::Value;

use crate::{EngineError, Interactive, Result, Session};

pub type Run = fn(&mut Session, &Value) -> Result<Value>;
pub type Enabled = fn(&Session) -> std::result::Result<(), String>;
pub type Factory = fn(&Session) -> Result<Box<dyn Interactive>>;

pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    /// Menu placement, e.g. `["Draw", "Circle", "Center, Radius"]`. Empty = not in menus.
    pub menu: &'static [&'static str],
    pub shortcut: Option<&'static str>,
    pub aliases: &'static [&'static str],
    pub params: &'static str,
    pub enabled: Enabled,
    pub run: Run,
    pub undoable: bool,
    pub interactive: Option<Factory>,
    /// Can run inside another command with a leading apostrophe.
    pub transparent: bool,
}

impl CommandSpec {
    pub const fn new(id: &'static str, label: &'static str, run: Run) -> Self {
        CommandSpec {
            id,
            label,
            menu: &[],
            shortcut: None,
            aliases: &[],
            params: "",
            enabled: has_doc,
            run,
            undoable: true,
            interactive: None,
            transparent: false,
        }
    }
    pub const fn menu(mut self, m: &'static [&'static str]) -> Self {
        self.menu = m;
        self
    }
    pub const fn key(mut self, k: &'static str) -> Self {
        self.shortcut = Some(k);
        self
    }
    pub const fn alias(mut self, a: &'static [&'static str]) -> Self {
        self.aliases = a;
        self
    }
    pub const fn params(mut self, p: &'static str) -> Self {
        self.params = p;
        self
    }
    pub const fn enabled(mut self, e: Enabled) -> Self {
        self.enabled = e;
        self
    }
    pub const fn noundo(mut self) -> Self {
        self.undoable = false;
        self
    }
    pub const fn interactive(mut self, f: Factory) -> Self {
        self.interactive = Some(f);
        self
    }
    pub const fn transparent(mut self) -> Self {
        self.transparent = true;
        self
    }
    pub fn info(&self, s: &Session) -> CommandInfo {
        let e = (self.enabled)(s);
        CommandInfo {
            id: self.id,
            label: self.label,
            menu: self.menu.to_vec(),
            shortcut: self.shortcut,
            aliases: self.aliases.to_vec(),
            params: self.params,
            interactive: self.interactive.is_some(),
            enabled: e.is_ok(),
            disabled_reason: e.err(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub menu: Vec<&'static str>,
    pub shortcut: Option<&'static str>,
    pub aliases: Vec<&'static str>,
    pub params: &'static str,
    pub interactive: bool,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

pub fn always(_: &Session) -> std::result::Result<(), String> {
    Ok(())
}
pub fn has_doc(s: &Session) -> std::result::Result<(), String> {
    s.state().map(|_| ()).map_err(|_| "no drawing open".into())
}
pub fn has_selection(s: &Session) -> std::result::Result<(), String> {
    has_doc(s)?;
    if s.selection().is_empty() { Err("nothing selected".into()) } else { Ok(()) }
}

pub fn command_specs() -> &'static [CommandSpec] {
    static SPECS: std::sync::OnceLock<Vec<CommandSpec>> = std::sync::OnceLock::new();
    SPECS.get_or_init(|| {
        let mut v = Vec::new();
        v.extend(buildercraft::specs());
        v.extend(closest3d::specs());
        v.extend(mesh_exchange::specs());
        v.extend(picking3d::specs());
        v.extend(snap3d::specs());
        v.extend(mesh3d::specs());
        v.extend(nurbs_document::specs());
        v.extend(brep3d::specs());
        v.extend(mesh_surface::specs());
        v.extend(feature_history::specs());
        v.extend(worldwright_tools::specs());
        v.extend(point3d::specs());
        v.extend(file::specs());
        v.extend(edit::specs());
        v.extend(qselect::specs());
        v.extend(view::specs());
        v.extend(draw::specs());
        v.extend(draw2::specs());
        v.extend(annotate::specs());
        v.extend(hatch::specs());
        v.extend(blocks::specs());
        v.extend(modify::specs());
        v.extend(modify2::specs());
        v.extend(gripcmds::specs());
        v.extend(layer::specs());
        v.extend(layout::specs());
        v.extend(props::specs());
        v.extend(inquiry::specs());
        v.extend(settings::specs());
        v.extend(utility::specs());
        v.extend(table::specs());
        v.extend(constraints::specs());
        v
    })
}

// Two sorted index arrays reuse the existing static registry rather than
// cloning command specs or creating per-command heap allocations. Sorting the
// alias index by original registration order preserves legacy alias precedence.
struct CommandIndex {
    by_id: Vec<usize>,
    by_alias: Vec<(&'static str, usize)>,
}

fn ascii_casefold_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.bytes().map(|byte| byte.to_ascii_lowercase()).cmp(b.bytes().map(|byte| byte.to_ascii_lowercase()))
}

fn command_index() -> &'static CommandIndex {
    static INDEX: std::sync::OnceLock<CommandIndex> = std::sync::OnceLock::new();
    INDEX.get_or_init(|| {
        let specs = command_specs();
        let mut by_id: Vec<usize> = (0..specs.len()).collect();
        by_id.sort_unstable_by(|a, b| specs[*a].id.cmp(specs[*b].id));

        let mut by_alias = Vec::new();
        for (index, spec) in specs.iter().enumerate() {
            for alias in spec.aliases {
                by_alias.push((*alias, index));
            }
        }
        by_alias.sort_unstable_by(|a, b| ascii_casefold_cmp(a.0, b.0).then_with(|| a.1.cmp(&b.1)));
        CommandIndex { by_id, by_alias }
    })
}

pub fn find_command(id: &str) -> Option<&'static CommandSpec> {
    let specs = command_specs();
    // The headless/API path already supplies lowercase IDs. Avoid allocating
    // a normalized String on its common execution path.
    let lookup = if id.bytes().any(|byte| byte.is_ascii_uppercase()) {
        std::borrow::Cow::Owned(id.to_ascii_lowercase())
    } else {
        std::borrow::Cow::Borrowed(id)
    };
    let indices = &command_index().by_id;
    let index = indices.binary_search_by(|&i| specs[i].id.cmp(lookup.as_ref())).ok()?;
    Some(&specs[indices[index]])
}

/// Resolve a typed name or alias (AutoCAD-style short names such as `L`, `C`, `PL`) to an id.
/// Exact command IDs always win, followed by the earliest registered alias.
pub fn resolve_alias(name: &str) -> String {
    if let Some(spec) = find_command(name) {
        return spec.id.to_owned();
    }
    let aliases = &command_index().by_alias;
    let index = aliases.partition_point(|(alias, _)| ascii_casefold_cmp(alias, name).is_lt());
    if let Some((alias, command_index)) = aliases.get(index)
        && alias.eq_ignore_ascii_case(name)
    {
        return command_specs()[*command_index].id.to_owned();
    }
    name.to_ascii_lowercase()
}

// ---------- param helpers ----------

pub(crate) fn bad(cmd: &str, msg: impl Into<String>) -> EngineError {
    EngineError::BadParams { cmd: cmd.into(), msg: msg.into() }
}
pub(crate) fn f64_or(p: &Value, key: &str, default: f64) -> f64 {
    p.get(key).and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(default)
}
pub(crate) fn f64_req(cmd: &str, p: &Value, key: &str) -> Result<f64> {
    p.get(key).and_then(Value::as_f64).filter(|v| v.is_finite()).ok_or_else(|| bad(cmd, format!("`{key}` (number) is required")))
}
pub(crate) fn bool_or(p: &Value, key: &str, default: bool) -> bool {
    p.get(key).and_then(Value::as_bool).unwrap_or(default)
}
pub(crate) fn str_param<'a>(p: &'a Value, key: &str) -> Option<&'a str> {
    p.get(key).and_then(Value::as_str)
}
pub fn point_value(v: &Value) -> Option<Vec2> {
    if let Some(a) = v.as_array() {
        let x = a.first()?.as_f64()?;
        let y = a.get(1)?.as_f64()?;
        let p = Vec2::new(x, y);
        return p.is_finite().then_some(p);
    }
    if let Some(s) = v.as_str() {
        return crate::prompt::parse_point(s, Vec2::ZERO);
    }
    let x = v.get("x")?.as_f64()?;
    let y = v.get("y")?.as_f64()?;
    let p = Vec2::new(x, y);
    p.is_finite().then_some(p)
}
pub(crate) fn point_param(p: &Value, key: &str) -> Option<Vec2> {
    p.get(key).and_then(point_value)
}
pub(crate) fn point_req(cmd: &str, p: &Value, key: &str) -> Result<Vec2> {
    point_param(p, key).ok_or_else(|| bad(cmd, format!("`{key}` ([x, y]) is required")))
}
pub(crate) fn points_param(p: &Value, key: &str) -> Option<Vec<Vec2>> {
    p.get(key)?.as_array()?.iter().map(point_value).collect()
}
/// Handles from `handles` (hex strings or numbers) or the current selection.
/// Explicitly supplied invalid handles must not silently fall back to selection
/// or shrink a batch: that could act on the wrong subset of a user's drawing.
pub(crate) fn targets(s: &Session, p: &Value) -> Result<Vec<Handle>> {
    let parse = |v: &Value| v.as_str().and_then(Handle::parse_hex).or_else(|| v.as_u64().map(Handle));
    if let Some(value) = p.get("handles") {
        let values = value.as_array().ok_or_else(|| bad("targets", "`handles` must be an array"))?;
        let handles: Vec<Handle> = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                parse(value).ok_or_else(|| bad("targets", format!("`handles[{index}]` must be a hexadecimal handle or unsigned integer")))
            })
            .collect::<Result<_>>()?;
        // Treat explicit targets as a set, like pick-first selections. Without
        // this, a repeated handle translates the same object multiple times.
        let mut unique = std::collections::HashSet::new();
        return Ok(handles.into_iter().filter(|handle| unique.insert(*handle)).collect());
    }
    if let Some(value) = p.get("handle") {
        return Ok(vec![parse(value).ok_or_else(|| bad("targets", "`handle` must be a hexadecimal handle or unsigned integer"))?]);
    }
    Ok(s.selection())
}
pub(crate) fn ok() -> Result<Value> {
    Ok(Value::Null)
}

#[cfg(test)]
mod command_index_tests {
    use super::*;

    fn linear_alias_reference(name: &str) -> String {
        let lower = name.to_ascii_lowercase();
        let specs = command_specs();
        if specs.iter().any(|command| command.id == lower) {
            return lower;
        }
        specs
            .iter()
            .find(|command| command.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(&lower)))
            .map(|command| command.id.to_owned())
            .unwrap_or(lower)
    }

    #[test]
    fn indexed_command_lookup_matches_the_prior_linear_path() {
        let specs = command_specs();
        assert!(!specs.is_empty());
        for spec in specs {
            for input in [spec.id.to_owned(), spec.id.to_ascii_uppercase()] {
                let expected = specs.iter().find(|command| command.id == input.to_ascii_lowercase());
                assert_eq!(find_command(&input).map(|command| command.id), expected.map(|command| command.id), "{input}");
                assert_eq!(resolve_alias(&input), linear_alias_reference(&input), "{input}");
            }
        }
        for spec in specs {
            for alias in spec.aliases {
                for input in [alias.to_string(), alias.to_ascii_uppercase(), alias.to_ascii_lowercase()] {
                    assert_eq!(resolve_alias(&input), linear_alias_reference(&input), "{input}");
                }
            }
        }
    }

    #[test]
    fn indexed_lookup_preserves_unknown_and_non_ascii_names() {
        for name in ["DOES_NOT_EXIST", "worldwright.no_such_command", "café.unknown", ""] {
            assert!(find_command(name).is_none(), "{name}");
            assert_eq!(resolve_alias(name), linear_alias_reference(name));
        }
    }

    #[test]
    fn index_uses_ordered_references_not_duplicate_command_specs() {
        let specs = command_specs();
        let index = command_index();
        assert_eq!(index.by_id.len(), specs.len());
        assert!(index.by_id.windows(2).all(|pair| specs[pair[0]].id <= specs[pair[1]].id));
        assert!(index.by_alias.windows(2).all(|pair| !ascii_casefold_cmp(pair[0].0, pair[1].0).is_gt()));
        let distinct: std::collections::HashSet<_> = specs.iter().map(|spec| spec.id).collect();
        assert_eq!(distinct.len(), specs.len(), "duplicate command IDs in registry");
    }
}
