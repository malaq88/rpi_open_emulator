use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use egui::{
    Align2, Color32, Context, Frame, Layout, Margin, RichText, Rounding, Sense, Stroke, TextureHandle,
    TextureOptions, Ui, Vec2,
};
use png::ColorType;

use crate::core::library::GameEntry;
use crate::theme;

pub struct GameCardOutput {
    pub play: bool,
    pub favorite_toggle: Option<bool>,
    pub open_details: bool,
}

const PH_W: usize = 128;
const PH_H: usize = 160;

fn hash_path(path: &Path) -> u64 {
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    h.finish()
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)).round() as u8
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        lerp_u8(a.r(), b.r(), t),
        lerp_u8(a.g(), b.g(), t),
        lerp_u8(a.b(), b.b(), t),
    )
}

fn placeholder_texture(
    ctx: &Context,
    cache_key: &Path,
    title: &str,
    cache: &mut HashMap<PathBuf, TextureHandle>,
) -> TextureHandle {
    if let Some(t) = cache.get(cache_key) {
        return t.clone();
    }
    let seed = hash_path(cache_key) ^ (title.len() as u64);
    let c0 = Color32::from_rgb(
        40 + ((seed >> 0) & 0x3f) as u8,
        50 + ((seed >> 8) & 0x4f) as u8,
        90 + ((seed >> 16) & 0x2f) as u8,
    );
    let c1 = Color32::from_rgb(
        20 + ((seed >> 24) & 0x2f) as u8,
        80 + ((seed >> 32) & 0x3f) as u8,
        120 + ((seed >> 40) & 0x2f) as u8,
    );
    let mut rgba = vec![0u8; PH_W * PH_H * 4];
    for y in 0..PH_H {
        for x in 0..PH_W {
            let t = (x + y) as f32 / (PH_W + PH_H) as f32;
            let c = lerp_color(c0, c1, t);
            let i = (y * PH_W + x) * 4;
            rgba[i] = c.r();
            rgba[i + 1] = c.g();
            rgba[i + 2] = c.b();
            rgba[i + 3] = 255;
        }
    }
    let img = egui::ColorImage::from_rgba_unmultiplied([PH_W, PH_H], &rgba);
    let tex = ctx.load_texture(
        format!("ph_{}", cache_key.to_string_lossy()),
        img,
        TextureOptions::LINEAR,
    );
    let out = tex.clone();
    cache.insert(cache_key.to_path_buf(), tex);
    out
}

fn decode_png_rgba(path: &Path) -> Option<(Vec<u8>, usize, usize)> {
    let ext = path.extension()?.to_str()?;
    if !ext.eq_ignore_ascii_case("png") {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let mut dec = png::Decoder::new(Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let w = info.width as usize;
    let h = info.height as usize;
    if w == 0 || h == 0 {
        return None;
    }
    let mut rgba = vec![0u8; w * h * 4];
    match info.color_type {
        ColorType::Rgba => {
            if info.line_size < w * 4 {
                return None;
            }
            for y in 0..h {
                let src = y * info.line_size;
                let dst = y * w * 4;
                rgba[dst..dst + w * 4].copy_from_slice(&buf[src..src + w * 4]);
            }
        }
        ColorType::Rgb => {
            if info.line_size < w * 3 {
                return None;
            }
            for y in 0..h {
                let src = y * info.line_size;
                for x in 0..w {
                    let si = src + x * 3;
                    let di = (y * w + x) * 4;
                    rgba[di] = buf[si];
                    rgba[di + 1] = buf[si + 1];
                    rgba[di + 2] = buf[si + 2];
                    rgba[di + 3] = 255;
                }
            }
        }
        _ => return None,
    }
    Some((rgba, w, h))
}

fn thumb_rgba(src: &[u8], w: usize, h: usize, max_w: usize, max_h: usize) -> (Vec<u8>, usize, usize) {
    let sx = max_w as f32 / w as f32;
    let sy = max_h as f32 / h as f32;
    let sc = sx.min(sy).min(1.0);
    let nw = ((w as f32 * sc).round() as usize).max(1);
    let nh = ((h as f32 * sc).round() as usize).max(1);
    if nw == w && nh == h {
        return (src.to_vec(), w, h);
    }
    let mut out = vec![0u8; nw * nh * 4];
    for y in 0..nh {
        let sy = ((y as f32 + 0.5) / nh as f32 * h as f32) as usize;
        let sy = sy.min(h - 1);
        for x in 0..nw {
            let sx = ((x as f32 + 0.5) / nw as f32 * w as f32) as usize;
            let sx = sx.min(w - 1);
            let si = (sy * w + sx) * 4;
            let di = (y * nw + x) * 4;
            out[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
    (out, nw, nh)
}

fn try_load_png_cover(
    ctx: &Context,
    path: &Path,
    cache: &mut HashMap<PathBuf, TextureHandle>,
) -> Option<TextureHandle> {
    if let Some(t) = cache.get(path) {
        return Some(t.clone());
    }
    let (rgba, w, h) = decode_png_rgba(path)?;
    let (rgba, w, h) = thumb_rgba(&rgba, w, h, 360, 420);
    let ci = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
    let tex = ctx.load_texture(
        format!("cover_{}", path.to_string_lossy()),
        ci,
        TextureOptions::LINEAR,
    );
    let t = tex.clone();
    cache.insert(path.to_path_buf(), tex);
    Some(t)
}

pub fn render_game_card(
    ui: &mut Ui,
    ctx: &Context,
    game: &GameEntry,
    cover_cache: &mut HashMap<PathBuf, TextureHandle>,
    card_width: f32,
    lazy_load: bool,
) -> GameCardOutput {
    let mut play = false;
    let mut favorite_toggle = None;
    let mut open_details = false;
    let card_id = egui::Id::new("game_card").with(game.path.to_string_lossy().to_string());
    let cover_h = card_width * 1.04;

    let inner = Frame::none()
        .fill(theme::BG_CARD)
        .rounding(Rounding::same(12.0))
        .stroke(Stroke::new(1.0, Color32::from_white_alpha(22)))
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.set_width(card_width);
            let cover_response =
                ui.allocate_response(Vec2::new(card_width - 8.0, cover_h), Sense::hover());
            let cover_rect = cover_response.rect;
            let visible = ui.clip_rect().intersects(cover_rect.expand(48.0));
            let hover_anim = ctx.animate_bool(card_id.with("hv"), cover_response.hovered());
            let glow = (22.0 + 50.0 * hover_anim) as u8;
            ui.painter().rect_filled(
                cover_rect,
                Rounding::same(10.0),
                Color32::from_rgb(15, 23, 42),
            );
            ui.painter().rect_stroke(
                cover_rect,
                Rounding::same(10.0),
                Stroke::new(1.0 + hover_anim, Color32::from_white_alpha(glow)),
            );

            ui.allocate_ui_at_rect(cover_rect.shrink(3.0), |ui| {
                let load = !lazy_load || visible;
                let mut drew = false;
                if load {
                    if let Some(ref cover_path) = game.cover_path {
                        if cover_path.exists() {
                            if let Some(tex) = try_load_png_cover(ctx, cover_path, cover_cache) {
                                let size = tex.size_vec2();
                                let max = cover_rect.size() - Vec2::splat(6.0);
                                let scale = (max.x / size.x).min(max.y / size.y);
                                let draw = size * scale;
                                ui.centered_and_justified(|ui| {
                                    ui.image((tex.id(), draw));
                                });
                                drew = true;
                            }
                        }
                    }
                }
                if !drew && load {
                    let ph_key = game
                        .cover_path
                        .as_ref()
                        .filter(|p| p.exists())
                        .cloned()
                        .unwrap_or_else(|| game.path.clone());
                    let tex = placeholder_texture(ctx, &ph_key, &game.title, cover_cache);
                    let size = tex.size_vec2();
                    let max = cover_rect.size() - Vec2::splat(6.0);
                    let scale = (max.x / size.x).min(max.y / size.y);
                    let draw = size * scale;
                    ui.centered_and_justified(|ui| {
                        ui.image((tex.id(), draw));
                    });
                    let ch = game
                        .title
                        .chars()
                        .find(|c| c.is_alphanumeric())
                        .map(|c| c.to_ascii_uppercase().to_string())
                        .unwrap_or_else(|| "?".to_string());
                    ui.painter().text(
                        cover_rect.center(),
                        Align2::CENTER_CENTER,
                        ch,
                        egui::FontId::proportional((card_width * 0.28).clamp(22.0, 44.0)),
                        Color32::from_white_alpha(180),
                    );
                    if game.cover_path.is_none() {
                        ui.painter().text(
                            cover_rect.center() + Vec2::new(0.0, cover_rect.height() * 0.22),
                            Align2::CENTER_CENTER,
                            "sem capa",
                            egui::FontId::proportional(11.0),
                            theme::TEXT_DIM,
                        );
                    }
                } else if !drew {
                    ui.with_layout(
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.label(
                                RichText::new("…")
                                    .strong()
                                    .size(13.0)
                                    .color(theme::TEXT_DIM),
                            );
                        },
                    );
                }

                if cover_response.hovered() && load {
                    ui.painter().rect_filled(
                        cover_rect,
                        Rounding::same(10.0),
                        Color32::from_black_alpha(72),
                    );
                }
            });

            ui.add_space(8.0);
            let title = if game.title.chars().count() > 40 {
                let mut t: String = game.title.chars().take(38).collect();
                t.push('…');
                t
            } else {
                game.title.clone()
            };
            ui.label(
                RichText::new(title)
                    .strong()
                    .size(13.0)
                    .color(theme::TEXT_MAIN),
            );
            ui.add_space(3.0);
            let sys_color = theme::accent_for_system(&game.system_key);
            ui.label(
                RichText::new(game.system_key.to_ascii_uppercase())
                    .small()
                    .strong()
                    .color(sys_color),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if theme::action_button(ui, "  Jogar  ").clicked() {
                    play = true;
                }
                ui.add_space(6.0);
                let fav_label = if game.is_favorite {
                    "★ Favorito"
                } else {
                    "☆ Favoritar"
                };
                if theme::outline_button(ui, fav_label, theme::WARM).clicked() {
                    favorite_toggle = Some(!game.is_favorite);
                }
                ui.add_space(6.0);
                if theme::outline_button(ui, "Detalhes", theme::ACCENT).clicked() {
                    open_details = true;
                }
            });
        });

    let _ = ctx.animate_bool(card_id.with("outer"), inner.response.hovered());
    GameCardOutput {
        play,
        favorite_toggle,
        open_details,
    }
}
