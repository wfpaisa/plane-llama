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

/// Aviso de que el editor modifica un script guardado, con su estado, el botón para
/// actualizarlo y la X para dejar de editarlo (el texto se queda en el editor).
fn editing_bar(ui: &mut egui::Ui, s: &mut Store, name: &str, changed: bool) {
    let t = Tokens::get(ui.ctx());
    egui::Frame::new()
        .fill(t.accent_soft)
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(egui::Margin::symmetric(Tokens::S3 as i8, Tokens::S2 as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(icons::PENCIL_SIMPLE)
                        .font(crate::theme::icon_font(Tokens::ICON))
                        .color(t.accent_ink),
                );
                ui.label(
                    egui::RichText::new("Editando")
                        .font(crate::theme::font(Tokens::FONT_BODY))
                        .color(t.text_dim),
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(name)
                            .font(crate::theme::font_semibold(Tokens::FONT_BODY))
                            .color(t.text),
                    )
                    .truncate(),
                );
                if changed {
                    widgets::chip(ui, "cambios sin guardar", t.warning_soft, t.warning);
                } else {
                    widgets::chip(ui, "guardado", t.surface_alt, t.text_dim);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::icon_button(ui, icons::X, "Dejar de editar este script").clicked() {
                        s.editor_source = None;
                    }
                    if widgets::enabled(ui, changed, |ui| {
                        widgets::button(
                            ui,
                            Kind::Secondary,
                            Some(icons::FLOPPY_DISK),
                            "Actualizar script",
                        )
                    })
                    .on_hover_text(format!("Guarda los cambios en «{name}»"))
                    .on_disabled_hover_text(format!("Sin cambios respecto a «{name}»"))
                    .clicked()
                    {
                        s.update_saved_from_editor();
                    }
                });
            });
        });
    ui.add_space(Tokens::S3);
}

pub fn show(ui: &mut egui::Ui, s: &mut Store) {
    // Script de la biblioteca abierto aquí y si el editor tiene cambios sin guardar en él.
    let saved = s.editor_saved().map(|x| {
        let changed = format_script(&s.script) != x.script;
        (x.name.clone(), changed)
    });
    widgets::card_marked(ui, saved.is_some(), |ui| {
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
                // Ligado a un script guardado, se puede copiar en uno nuevo (actualizar va en la
                // franja «Editando»); si no, se guarda como script.
                match &saved {
                    Some(_) => {
                        if widgets::button(
                            ui,
                            Kind::Secondary,
                            Some(icons::FILE_PLUS),
                            "Guardar como nuevo",
                        )
                        .on_hover_text("Crea otro script con el texto del editor")
                        .clicked()
                        {
                            s.save_editor_as_new();
                        }
                    }
                    None => {
                        if widgets::button(
                            ui,
                            Kind::Secondary,
                            Some(icons::FILE_PLUS),
                            "Guardar como script",
                        )
                        .on_hover_text("Lo agrega a tus scripts con un nombre")
                        .clicked()
                        {
                            s.save_editor_as_new();
                        }
                    }
                }
                let pick = widgets::icon_button(ui, icons::CUBE, "Elegir modelo descargado");
                if let Some(crate::models_page::Picked::Model(hf)) =
                    crate::models_page::picker(ui, s, &pick)
                {
                    s.script = plane_llama_core::models::script_with_model(&s.script, &hf);
                    s.script_seeded = true;
                }
                if widgets::icon_button(ui, icons::BOOK_OPEN_TEXT, "Catálogo de flags").clicked() {
                    s.flags_open = true;
                }
                let menu = widgets::icon_button(ui, icons::DOTS_THREE, "Más acciones del script");
                egui::Popup::menu(&menu)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                    .show(|ui| {
                        let item = |ui: &mut egui::Ui, icon, label| {
                            widgets::button(ui, Kind::Ghost, Some(icon), label).clicked()
                        };
                        if item(ui, icons::TEXT_INDENT, "Formatear") {
                            s.script = format_script(&s.script);
                            s.notice("Script formateado");
                        }
                        widgets::separator(ui);
                        if item(ui, icons::PUSH_PIN, "Guardar como script por defecto") {
                            s.confirm(
                                "¿Guardar el script actual como default?",
                                Action::SaveScript(s.script.clone()),
                            );
                        }
                        if item(
                            ui,
                            icons::ARROW_COUNTER_CLOCKWISE,
                            "Volver al script por defecto",
                        ) {
                            s.confirm(
                                "¿Restablecer el script al default guardado? Se perderán los cambios no guardados.",
                                Action::ScriptDefault,
                            );
                        }
                    });
            },
        );
        if let Some((name, changed)) = &saved {
            editing_bar(ui, s, name, *changed);
        }
        let pasted = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))));
        let edited = code_box(
            ui,
            "script-editor",
            &mut s.script,
            Tokens::EDITOR_ROWS,
            "llama-server -hf org/modelo:Q4_K_M --ctx-size 8192 …",
        );
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

/// Caja de edición de un script: números de línea, flags resaltados y borde de acento
/// con el foco. La usan el Editor y el diálogo de scripts.
pub fn code_box(
    ui: &mut egui::Ui,
    id_salt: &str,
    script: &mut String,
    rows: usize,
    hint: &str,
) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let id = ui.make_persistent_id(id_salt);
    let focused = ui.memory(|m| m.has_focus(id));
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let digits = script.lines().count().max(1).to_string().len().max(2) as f32;
    let gutter = ui.fonts_mut(|f| f.glyph_width(&font, '0')) * digits + Tokens::S3;
    // El grosor del borde no cambia con el foco: el Frame lo suma al tamaño y desplazaría
    // el contenido; el realce va como un trazo pintado encima, hacia dentro.
    let color = if focused { t.accent } else { t.border };
    let frame = egui::Frame::new()
        .fill(t.sunken)
        .stroke(egui::Stroke::new(Tokens::BORDER, color))
        .corner_radius(Tokens::RADIUS_MD)
        .inner_margin(egui::Margin {
            left: Tokens::S2 as i8,
            right: Tokens::S3 as i8,
            top: Tokens::S3 as i8,
            bottom: Tokens::S3 as i8,
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                let (gutter_rect, _) =
                    ui.allocate_exact_size(egui::vec2(gutter, 0.), egui::Sense::hover());
                let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| {
                    let mut job = highlight(text.as_str(), &font, &t);
                    job.wrap.max_width = wrap;
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let output = egui::TextEdit::multiline(script)
                    .id(id)
                    .code_editor()
                    .frame(egui::Frame::new())
                    .hint_text(hint)
                    .desired_rows(rows)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter)
                    .show(ui);
                // Un número por línea del script, en la fila donde empieza (con ajuste
                // de línea, las filas siguientes no llevan número).
                let mut line = 1;
                let mut starts = true;
                for row in &output.galley.rows {
                    if starts {
                        ui.painter().text(
                            egui::pos2(
                                gutter_rect.right() - Tokens::S2,
                                output.galley_pos.y + row.rect().min.y,
                            ),
                            egui::Align2::RIGHT_TOP,
                            line.to_string(),
                            font.clone(),
                            t.text_faint,
                        );
                        line += 1;
                    }
                    starts = row.ends_with_newline;
                }
                output.response.response
            })
            .inner
        });
    if focused {
        ui.painter().rect_stroke(
            frame.response.rect,
            Tokens::RADIUS_MD,
            egui::Stroke::new(Tokens::BORDER + 1., t.accent),
            egui::StrokeKind::Inside,
        );
    }
    frame.inner
}

/// Colores del script: binario atenuado, flags en acento y `\` de continuación tenue.
fn highlight(text: &str, font: &egui::FontId, t: &Tokens) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let format = |color| egui::TextFormat::simple(font.clone(), color);
    let mut first = true;
    let mut rest = text;
    while !rest.is_empty() {
        let space = rest
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(rest.len());
        if space > 0 {
            job.append(&rest[..space], 0., format(t.text));
            rest = &rest[space..];
            continue;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let word = &rest[..end];
        let color = if first {
            t.text_dim
        } else if word == "\\" {
            t.text_faint
        } else if is_flag(word) {
            t.accent_ink
        } else {
            t.text
        };
        first = false;
        job.append(word, 0., format(color));
        rest = &rest[end..];
    }
    job
}
/// `-x` o `--flag`, pero no un número negativo.
fn is_flag(word: &str) -> bool {
    word.len() > 1
        && word.starts_with('-')
        && !word[1..].starts_with(|c: char| c.is_ascii_digit() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flags_are_not_negative_numbers() {
        assert!(is_flag("--ctx-size"));
        assert!(is_flag("-hf"));
        assert!(!is_flag("-1"));
        assert!(!is_flag("-"));
        assert!(!is_flag("q8_0"));
    }
}
