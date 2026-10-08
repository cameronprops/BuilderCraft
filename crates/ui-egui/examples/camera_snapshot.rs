//! Headless capture of actual egui viewport shapes for camera visual review.
use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services};
use egui::{Color32, epaint::Shape};
use serde_json::json;
fn color(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}
fn shape(s: &Shape, out: &mut String) {
    use std::fmt::Write;
    match s {
        Shape::Vec(v) => {
            for s in v {
                shape(s, out)
            }
        }
        Shape::LineSegment { points, stroke } => {
            let _ = write!(
                out,
                "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='{}' stroke-width='{}'/>",
                points[0].x,
                points[0].y,
                points[1].x,
                points[1].y,
                color(stroke.color),
                stroke.width
            );
        }
        Shape::Circle(c) => {
            let _ = write!(out, "<circle cx='{}' cy='{}' r='{}' fill='{}'/>", c.center.x, c.center.y, c.radius, color(c.fill));
        }
        Shape::Rect(r) if r.fill.a() > 0 => {
            let _ = write!(
                out,
                "<rect x='{}' y='{}' width='{}' height='{}' rx='3' fill='{}'/>",
                r.rect.min.x,
                r.rect.min.y,
                r.rect.width(),
                r.rect.height(),
                color(r.fill)
            );
        }
        Shape::Text(t) => {
            // Preserve egui's wrapping and glyph positions while substituting a system font.
            for row in &t.galley.rows {
                for glyph in &row.glyphs {
                    let text = glyph.chr.to_string().replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
                    let _ = write!(
                        out,
                        "<text x='{}' y='{}' fill='{}' font-size='{}' font-family='DejaVu Sans'>{}</text>",
                        t.pos.x + row.pos.x + glyph.pos.x,
                        t.pos.y + row.pos.y + glyph.pos.y,
                        color(t.fallback_color),
                        glyph.font_height,
                        text
                    );
                }
            }
        }
        _ => {}
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).ok_or("output directory required")?;
    let height = std::env::args().nth(2).map(|s| s.parse::<f32>()).transpose()?.unwrap_or(700.);
    if !height.is_finite() || !(400. ..=2000.).contains(&height) {
        return Err("capture height must be between 400 and 2000 pixels".into());
    }
    std::fs::create_dir_all(&output)?;
    let mut app = CadApp::new(Session::new(), Services::default());
    let curve = app.run("ui.buildercraft.curve", json!({}))?;
    app.session.set_selection(vec![cadcraft_doc::Handle(curve["id"].as_u64().ok_or("curve ID missing")?)]);
    app.run("ui.buildercraft.surface", json!({}))?;
    app.session.viewport_px = (1000., 650.);
    let ctx = egui::Context::default();
    for name in ["top", "front", "right", "iso"] {
        app.run(&format!("ui.buildercraft.{name}"), json!({}))?;
        app.run("ui.buildercraft.fit", json!({}))?;
        let input =
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000., height))), ..Default::default() };
        let mut initial = ctx.run_ui(input.clone(), |ui| {
            app.logic(ui.ctx());
            app.ui(ui)
        });
        initial.textures_delta.clear(); // Vector capture deliberately does not upload textures.
        app.run("ui.buildercraft.fit", json!({}))?;
        let mut frame = ctx.run_ui(input, |ui| {
            app.logic(ui.ctx());
            app.ui(ui)
        });
        frame.textures_delta.clear();
        let mut svg =
            format!("<svg xmlns='http://www.w3.org/2000/svg' width='1000' height='{height}'><rect width='1000' height='{height}' fill='#181b21'/>");
        for (i, clipped) in frame.shapes.into_iter().enumerate() {
            use std::fmt::Write;
            let r = clipped.clip_rect;
            let _ = write!(
                svg,
                "<defs><clipPath id='clip{i}'><rect x='{}' y='{}' width='{}' height='{}'/></clipPath></defs><g clip-path='url(#clip{i})'>",
                r.min.x,
                r.min.y,
                r.width(),
                r.height()
            );
            shape(&clipped.shape, &mut svg);
            svg.push_str("</g>");
        }
        svg.push_str("</svg>");
        std::fs::write(std::path::Path::new(&output).join(format!("{name}.svg")), svg)?;
    }
    Ok(())
}
