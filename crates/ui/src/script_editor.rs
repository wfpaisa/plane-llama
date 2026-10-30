// Tarjeta del script de llama-server y catálogo de flags para agregarlos con un clic.
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind, icons};
use crate::{bridge::Action, store::Store};
use egui::RichText;
use plane_llama_core::{
    client::{
        flags_catalog::{LlamaFlag, flags_catalog},
        format::{fmt, format_script, short_model},
    },
    script_parser::{parse_script, tokenize_script},
};

pub const CATEGORIES: [&str; 5] = ["Todas", "Común", "Muestreo", "Especulativo", "Servidor"];

/// Resumen del script en chips: modelo, contexto, capas y puerto.
pub fn summary(ui: &mut egui::Ui, script: &str) {
    let t = Tokens::get(ui.ctx());
    let Ok(p) = parse_script(script) else {
        widgets::dim(ui, "El script no tiene un comando válido todavía.");
        return;
    };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(Tokens::S1 + 2., Tokens::S1 + 2.);
        if p.model.is_some() {
            widgets::chip(
                ui,
                &short_model(p.model.as_deref()),
                t.accent_soft,
                t.accent_ink,
            );
        }
        for (label, value) in [("ctx", p.ctx_size), ("ngl", p.ngl), ("batch", p.batch_size)] {
            if value.is_some() {
                widgets::chip(
                    ui,
                    &format!("{label} {}", fmt(value, 0)),
                    t.surface_alt,
                    t.text_dim,
                );
            }
        }
        if p.flash_attn {
            widgets::chip(ui, "flash-attn", t.surface_alt, t.text_dim);
        }
        widgets::chip(
            ui,
            &format!("{}:{}", p.host, p.port),
            t.surface_alt,
            t.text_dim,
        );
    });
}

pub fn show(ui: &mut egui::Ui, s: &mut Store) {
    widgets::card(ui, |ui| {
        widgets::card_header(
            ui,
            icons::CODE,
            "Script del servidor",
            Some("Comando de llama-server"),
            |ui| {
                if widgets::button(ui, Kind::Primary, Some(icons::LIGHTNING), "Optimizar")
                    .on_hover_text("Ajustar parámetros según la VRAM")
                    .clicked()
                {
                    crate::optimizer_modal::open(s);
                }
                if widgets::icon_button(ui, icons::BOOK_OPEN_TEXT, "Catálogo de flags").clicked() {
                    s.flags_open = true;
                }
                let pick = widgets::icon_button(ui, icons::CUBE, "Elegir modelo descargado");
                if let Some(crate::models_page::Picked::Model(hf)) =
                    crate::models_page::picker(ui, s, &pick)
                {
                    s.script = plane_llama_core::models::script_with_model(&s.script, &hf);
                    s.script_seeded = true;
                }
                if widgets::icon_button(ui, icons::BOOKMARK_SIMPLE, "Guardar en mis scripts")
                    .clicked()
                {
                    s.library.draft = Some(crate::library::Draft {
                        id: None,
                        name: String::new(),
                        script: s.script.clone(),
                    });
                }
                if widgets::icon_button(ui, icons::ARROW_COUNTER_CLOCKWISE, "Obtener guardado")
                    .clicked()
                {
                    s.confirm(
                        "¿Restablecer el script al default guardado? Se perderán los cambios no guardados.",
                        Action::ScriptDefault,
                    );
                }
                if widgets::icon_button(ui, icons::FLOPPY_DISK, "Guardar").clicked() {
                    s.confirm(
                        "¿Guardar el script actual como default?",
                        Action::SaveScript(s.script.clone()),
                    );
                }
                if widgets::icon_button(ui, icons::TEXT_INDENT, "Formatear").clicked() {
                    s.script = format_script(&s.script);
                    s.notice("Script formateado");
                }
            },
        );
        let pasted = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))));
        let edited = widgets::sunken(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut s.script)
                    .code_editor()
                    .frame(egui::Frame::new())
                    .hint_text("llama-server -hf org/modelo:Q4_K_M --ctx-size 8192 …")
                    .desired_rows(Tokens::EDITOR_ROWS)
                    .desired_width(f32::INFINITY),
            )
        });
        if edited.changed() {
            s.script_seeded = true;
            if pasted {
                s.script = format_script(&s.script);
            }
        }
        ui.add_space(Tokens::S1);
        summary(ui, &s.script);
    });
}

fn matches(f: &LlamaFlag, search: &str) -> bool {
    search.is_empty()
        || format!(
            "{} {} {} {} {}",
            f.name,
            f.long,
            f.short.as_deref().unwrap_or(""),
            f.description,
            f.original_description.as_deref().unwrap_or("")
        )
        .to_lowercase()
        .contains(search)
}

fn flag_row(ui: &mut egui::Ui, s: &mut Store, f: &LlamaFlag, present: bool) {
    let t = Tokens::get(ui.ctx());
    egui::Frame::new()
        .fill(t.surface)
        .stroke(egui::Stroke::new(Tokens::BORDER, t.border))
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(Tokens::S3)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&f.long)
                        .font(egui::FontId::monospace(Tokens::FONT_BODY))
                        .color(t.accent_ink),
                );
                if let Some(short) = &f.short {
                    widgets::chip(ui, short, t.surface_alt, t.text_dim);
                }
                if let Some(value) = &f.default_value {
                    widgets::chip(ui, &format!("default {value}"), t.surface_alt, t.text_dim);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if present {
                        widgets::chip(ui, "En el script", t.ok_soft, t.ok);
                    } else if widgets::button(ui, Kind::Secondary, Some(icons::PLUS), "Agregar")
                        .clicked()
                    {
                        if !s.script.trim().is_empty() {
                            s.script.push_str(" \\\n  ");
                        }
                        s.script.push_str(&f.long);
                        if let Some(value) = &f.default_value {
                            s.script.push(' ');
                            s.script.push_str(value);
                        }
                        s.script = format_script(&s.script);
                        s.notice(format!("Flag {} agregado", f.long));
                    }
                    let favorite = s.favorites.contains(&f.long);
                    let star = ui
                        .add(
                            egui::Label::new(
                                RichText::new(if favorite { "★" } else { "☆" })
                                    .font(theme::font(Tokens::ICON + 1.))
                                    .color(if favorite { t.warning } else { t.text_faint }),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_text(if favorite {
                            "Quitar de destacados"
                        } else {
                            "Destacar"
                        });
                    if star.clicked() {
                        if favorite {
                            s.favorites.retain(|x| x != &f.long);
                        } else {
                            s.favorites.push(f.long.clone());
                        }
                        s.actions.push(Action::Favorites(s.favorites.clone()));
                    }
                });
            });
            ui.add(
                egui::Label::new(
                    RichText::new(&f.description)
                        .font(theme::font(Tokens::FONT_SM + 0.5))
                        .color(t.text),
                )
                .wrap(),
            );
            if let Some(original) = &f.original_description {
                ui.add(
                    egui::Label::new(
                        RichText::new(original)
                            .font(theme::font(Tokens::FONT_XS + 0.5))
                            .color(t.text_faint),
                    )
                    .wrap(),
                );
            }
        });
}

/// Catálogo de flags en un diálogo: búsqueda, categorías y destacados.
pub fn catalog(ctx: &egui::Context, s: &mut Store) {
    if !s.flags_open {
        return;
    }
    let dialog = widgets::dialog(
        ctx,
        "flags-catalog",
        "Catálogo de flags",
        Tokens::DIALOG_LG,
        |ui| {
            ui.horizontal_wrapped(|ui| {
                widgets::search(
                    ui,
                    &mut s.flag_search,
                    "Buscar flag o descripción",
                    Tokens::SEARCH_WIDTH,
                );
                let mut category = s.flag_category;
                let options: Vec<_> = CATEGORIES
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (i, *c))
                    .collect();
                if widgets::segmented(ui, &mut category, &options) {
                    s.flag_category = category;
                }
                widgets::toggle(
                    ui,
                    &mut s.favorites_only,
                    &format!("Solo destacados ({})", s.favorites.len()),
                );
            });
            ui.add_space(Tokens::S2);
            let search = s.flag_search.to_lowercase();
            let tokens = tokenize_script(&s.script).unwrap_or_default();
            let flags: Vec<_> = flags_catalog()
                .iter()
                .filter(|f| s.flag_category == 0 || f.category == CATEGORIES[s.flag_category])
                .filter(|f| !s.favorites_only || s.favorites.contains(&f.long))
                .filter(|f| matches(f, &search))
                .collect();
            widgets::dim(ui, format!("{} flags", flags.len()));
            let height = (ctx.content_rect().height() - Tokens::DIALOG_RESERVED * 1.5)
                .max(Tokens::DIALOG_MIN);
            egui::ScrollArea::vertical()
                .id_salt("flags-list")
                .auto_shrink([false, false])
                .min_scrolled_height(height)
                .max_height(height)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = Tokens::S2;
                    if flags.is_empty() {
                        widgets::empty_state(
                            ui,
                            icons::MAGNIFYING_GLASS,
                            "Sin coincidencias",
                            "Prueba con otra palabra o cambia de categoría.",
                        );
                    }
                    for f in flags {
                        let present = tokens.iter().any(|t| {
                            t == &f.long
                                || f.short.as_ref() == Some(t)
                                || f.aliases.as_ref().is_some_and(|a| a.contains(t))
                                || t.starts_with(&format!("{}=", f.long))
                        });
                        ui.push_id(&f.long, |ui| flag_row(ui, s, f, present));
                    }
                });
        },
    );
    if dialog.close {
        s.flags_open = false;
    }
}
