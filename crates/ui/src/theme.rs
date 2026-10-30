// Tokens de diseño: paletas, tipografía y proporciones de toda la interfaz.
use egui::{Color32, FontFamily, FontId, Stroke};

#[derive(Clone, Copy)]
pub struct Tokens {
    /// Fondo del área de contenido.
    pub bg: Color32,
    /// Fondo de la barra lateral de navegación.
    pub sidebar: Color32,
    /// Tarjetas y diálogos.
    pub surface: Color32,
    /// Cabeceras de tabla, chips y botones secundarios.
    pub surface_alt: Color32,
    /// Controles hundidos: inputs, editor y logs.
    pub sunken: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub hover: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub accent_ink: Color32,
    pub on_accent: Color32,
    pub ok: Color32,
    pub ok_soft: Color32,
    pub warning: Color32,
    pub warning_soft: Color32,
    pub error: Color32,
    pub error_soft: Color32,
    /// Segmentos del consumo de VRAM: pesos, KV cache, overhead y VRAM ya en uso.
    pub series: [Color32; 4],
    /// Grupos de métricas del historial: escritura, lectura y especulativo.
    pub groups: [Color32; 3],
    pub shadow: Color32,
    /// Borde exterior de la ventana, algo más claro que el fondo.
    pub window_border: Color32,
    pub backdrop: Color32,
    pub transparent: Color32,
}
impl Tokens {
    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(21, 22, 28),
            sidebar: Color32::from_rgb(16, 17, 22),
            surface: Color32::from_rgb(28, 30, 37),
            surface_alt: Color32::from_rgb(36, 38, 47),
            sunken: Color32::from_rgb(18, 19, 24),
            border: Color32::from_rgb(41, 44, 54),
            border_strong: Color32::from_rgb(56, 60, 73),
            hover: Color32::from_rgb(40, 43, 53),
            text: Color32::from_rgb(233, 235, 241),
            text_dim: Color32::from_rgb(154, 160, 176),
            text_faint: Color32::from_rgb(105, 111, 128),
            accent: Color32::from_rgb(124, 108, 246),
            accent_hover: Color32::from_rgb(143, 129, 250),
            accent_soft: Color32::from_rgba_unmultiplied(124, 108, 246, 42),
            accent_ink: Color32::from_rgb(196, 188, 255),
            on_accent: Color32::WHITE,
            ok: Color32::from_rgb(52, 211, 153),
            ok_soft: Color32::from_rgba_unmultiplied(52, 211, 153, 32),
            warning: Color32::from_rgb(251, 191, 36),
            warning_soft: Color32::from_rgba_unmultiplied(251, 191, 36, 32),
            error: Color32::from_rgb(248, 113, 113),
            error_soft: Color32::from_rgba_unmultiplied(248, 113, 113, 34),
            groups: [
                Color32::from_rgb(56, 132, 248),
                Color32::from_rgb(150, 92, 246),
                Color32::from_rgb(232, 136, 40),
            ],
            series: [
                Color32::from_rgb(124, 108, 246),
                Color32::from_rgb(56, 189, 248),
                Color32::from_rgb(244, 114, 182),
                Color32::from_rgb(88, 94, 110),
            ],
            shadow: Color32::from_black_alpha(90),
            window_border: Color32::from_white_alpha(34),
            backdrop: Color32::from_black_alpha(150),
            transparent: Color32::TRANSPARENT,
        }
    }
    pub fn light() -> Self {
        Self {
            bg: Color32::from_rgb(247, 248, 250),
            sidebar: Color32::from_rgb(239, 240, 244),
            surface: Color32::WHITE,
            surface_alt: Color32::from_rgb(243, 244, 247),
            sunken: Color32::from_rgb(248, 249, 251),
            border: Color32::from_rgb(226, 229, 235),
            border_strong: Color32::from_rgb(208, 212, 221),
            hover: Color32::from_rgb(234, 236, 242),
            text: Color32::from_rgb(24, 26, 33),
            text_dim: Color32::from_rgb(90, 97, 112),
            text_faint: Color32::from_rgb(140, 146, 160),
            accent: Color32::from_rgb(99, 82, 232),
            accent_hover: Color32::from_rgb(84, 67, 218),
            accent_soft: Color32::from_rgba_unmultiplied(99, 82, 232, 30),
            accent_ink: Color32::from_rgb(78, 60, 205),
            on_accent: Color32::WHITE,
            ok: Color32::from_rgb(5, 150, 105),
            ok_soft: Color32::from_rgba_unmultiplied(5, 150, 105, 28),
            warning: Color32::from_rgb(196, 132, 4),
            warning_soft: Color32::from_rgba_unmultiplied(234, 179, 8, 36),
            error: Color32::from_rgb(220, 38, 38),
            error_soft: Color32::from_rgba_unmultiplied(220, 38, 38, 26),
            groups: [
                Color32::from_rgb(37, 99, 235),
                Color32::from_rgb(124, 58, 237),
                Color32::from_rgb(217, 119, 6),
            ],
            series: [
                Color32::from_rgb(99, 82, 232),
                Color32::from_rgb(14, 165, 233),
                Color32::from_rgb(219, 39, 119),
                Color32::from_rgb(170, 176, 190),
            ],
            shadow: Color32::from_black_alpha(22),
            window_border: Color32::from_black_alpha(46),
            backdrop: Color32::from_black_alpha(70),
            transparent: Color32::TRANSPARENT,
        }
    }
    /// Color que `Painter::galley` reemplaza por el color de tinta que se le pase.
    pub const INK: Color32 = Color32::PLACEHOLDER;

    /// Ventana de la vista ampliada; nunca más grande que el monitor (`MONITOR_MARGIN`).
    pub const WINDOW_SIZE: [f32; 2] = [1600., 1000.];
    pub const WINDOW_MIN: [f32; 2] = [880., 560.];
    /// Ventana compacta de la vista simple.
    pub const COMPACT_SIZE: [f32; 2] = [900., 600.];
    pub const COMPACT_MIN: [f32; 2] = [360., 260.];
    /// Lo que se deja libre del monitor al limitar la ventana: barra superior y sombra.
    pub const MONITOR_MARGIN: f32 = 64.;
    /// Margen transparente alrededor de la ventana donde se pinta su sombra.
    pub const WINDOW_SHADOW: f32 = 14.;
    pub const WINDOW_RADIUS: u8 = 12;
    /// Opacidad del fondo con el desenfoque activado.
    pub const GLASS_ALPHA: f32 = 0.76;

    pub const S1: f32 = 4.;
    pub const S2: f32 = 8.;
    pub const S3: f32 = 12.;
    pub const S4: f32 = 16.;
    pub const S5: f32 = 20.;
    pub const S6: f32 = 24.;
    pub const S8: f32 = 32.;

    pub const RADIUS_SM: u8 = 6;
    pub const RADIUS_MD: u8 = 8;
    pub const RADIUS_LG: u8 = 12;
    pub const RADIUS_FULL: u8 = 255;

    pub const FONT_XS: f32 = 11.;
    pub const FONT_SM: f32 = 12.;
    pub const FONT_BODY: f32 = 13.5;
    pub const FONT_MD: f32 = 15.;
    pub const FONT_LG: f32 = 18.;
    pub const FONT_XL: f32 = 22.;
    pub const FONT_KPI: f32 = 28.;
    pub const TRACKING: f32 = 0.06;

    pub const BORDER: f32 = 1.;
    pub const CONTROL: f32 = 30.;
    pub const CONTROL_LG: f32 = 40.;
    pub const ICON: f32 = 16.;
    pub const ICON_LG: f32 = 20.;
    pub const DOT: f32 = 4.;
    pub const BAR: f32 = 6.;
    pub const BAR_LG: f32 = 12.;
    pub const TOGGLE: [f32; 2] = [32., 18.];

    pub const SIDEBAR: f32 = 232.;
    pub const SIDEBAR_COLLAPSED: f32 = 64.;
    pub const NAV_ITEM: f32 = 36.;
    pub const TOPBAR: f32 = 56.;
    pub const CAPTION: [f32; 2] = [32., 30.];
    /// Diámetro del círculo de los botones de ventana.
    pub const CAPTION_DOT: f32 = 24.;
    pub const RESIZE: f32 = 6.;
    /// Lado de las zonas de las esquinas, por dentro del margen de la sombra.
    pub const RESIZE_CORNER: f32 = 18.;
    /// Ancho del contenido a partir del cual las páginas usan dos columnas.
    pub const TWO_COLUMNS: f32 = 900.;
    pub const SIDE_COLUMN: f32 = 0.38;

    pub const DIALOG_INSET: f32 = 64.;
    pub const DIALOG_MIN: f32 = 320.;
    pub const DIALOG_SM: f32 = 440.;
    pub const DIALOG_MD: f32 = 560.;
    pub const DIALOG_LG: f32 = 1100.;
    pub const DIALOG_RESERVED: f32 = 190.;
    /// Alto de la cabecera del diálogo y del margen que el modal deja con la ventana.
    pub const DIALOG_CHROME: f32 = 160.;
    pub const DIALOG_FOOTER: f32 = 88.;
    pub const TOAST_WIDTH: f32 = 360.;
    pub const SEARCH_WIDTH: f32 = 240.;
    pub const SEARCH_NARROW: f32 = 150.;
    /// Ancho a partir del cual las barras de herramientas muestran todas las etiquetas.
    pub const TOOLBAR_FULL: f32 = 840.;
    pub const EDITOR_ROWS: usize = 16;
    pub const PROMPT_ROWS: usize = 6;
    pub const RESPONSE_HEIGHT: f32 = 320.;
    pub const LOG_ROW: f32 = 18.;
    pub const LOG_GUTTER: f32 = 56.;
    pub const ROW: f32 = 36.;
    pub const TABLE_HEADER: f32 = 34.;
    pub const CHECK_WIDTH: f32 = 36.;
    pub const INDEX_WIDTH: f32 = 36.;
    pub const ACTIONS_WIDTH: f32 = 44.;
    /// Columna de acciones del historial: cargar en el editor y guardar como script.
    pub const HISTORY_ACTIONS: f32 = 76.;
    pub const MODEL_WIDTH: f32 = 260.;
    pub const RATING_WIDTH: f32 = 150.;
    pub const COLUMN_WIDTH: f32 = 120.;
    pub const COLUMN_MIN: f32 = 48.;
    /// Margen a cada lado del contenido de una celda de tabla.
    pub const CELL_PAD: f32 = 8.;
    pub const STAT_MIN: f32 = 150.;
    pub const COMPARE_LABEL: f32 = 150.;
    pub const COMPARE_COLUMN: f32 = 170.;
    pub const CHART_HEIGHT: f32 = 420.;
    pub const VALUE_WIDTH: f32 = 72.;
    /// Espacio del botón «Recargar dispositivos» junto a los datos del modelo.
    pub const RELOAD_WIDTH: f32 = 220.;
    pub const ROW_LG: f32 = 40.;
    pub const ROW_HUB: f32 = 56.;
    pub const CHIP_SLOT: f32 = 76.;
    pub const ICON_SLOT: f32 = 20.;
    pub const QUANT_SLOT: f32 = 112.;
    pub const TPS_SLOT: f32 = 100.;
    pub const ACTION_SLOT: f32 = 104.;
    pub const HUB_LIST: f32 = 380.;
    pub const LOGO: f32 = 28.;
    pub const SWATCH: f32 = 24.;
    pub const SIZE_WIDTH: f32 = 64.;
    pub const DIALOG_XL: f32 = 760.;
    pub const COMPACT_BAR: f32 = 40.;
    /// Ancho de fila por debajo del cual Scripts deja los botones solo con icono.
    pub const ROW_NARROW: f32 = 600.;
    /// Anchos de fila a partir de los cuales Scripts muestra la cuantización, y los
    /// tok/s y el icono de visión; la ventana compacta queda por debajo de ambos.
    pub const ROW_QUANT: f32 = 380.;
    pub const ROW_TPS: f32 = 480.;
    /// Anchos de fila a partir de los cuales Scripts muestra tamaño, y contexto y parámetros.
    pub const ROW_WIDE: f32 = 640.;
    pub const ROW_WIDER: f32 = 760.;

    pub fn get(ctx: &egui::Context) -> Self {
        WEIGHTS.set(ctx.fonts(|f| f.definitions().families.contains_key(&semibold())));
        Self::of(ctx.theme())
    }
    /// Paleta de un tema con el color de acento elegido en Ajustes.
    pub fn of(theme: egui::Theme) -> Self {
        let dark = theme == egui::Theme::Dark;
        let mut t = if dark { Self::dark() } else { Self::light() };
        let accent = accent();
        if accent != Accent::default() {
            t.with_accent(accent.color(dark), dark);
        }
        if glass() {
            for c in [&mut t.bg, &mut t.sidebar] {
                *c = c.gamma_multiply(Self::GLASS_ALPHA);
            }
        }
        t
    }
    fn with_accent(&mut self, c: Color32, dark: bool) {
        let luma = 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
        self.accent = c;
        self.accent_hover = if dark {
            c.lerp_to_gamma(Color32::WHITE, 0.14)
        } else {
            c.lerp_to_gamma(Color32::BLACK, 0.12)
        };
        self.accent_soft =
            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), if dark { 42 } else { 30 });
        self.accent_ink = if dark {
            c.lerp_to_gamma(Color32::WHITE, 0.5)
        } else {
            c.lerp_to_gamma(Color32::BLACK, 0.22)
        };
        // Acentos claros (ámbar, cian) llevan texto oscuro encima.
        self.on_accent = if luma > 165. {
            Color32::from_rgb(20, 21, 26)
        } else {
            Color32::WHITE
        };
        self.series[0] = c;
    }
}

/// Color de acento: uno de `ACCENTS` o uno propio elegido con el selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Accent {
    Preset(usize),
    Custom([u8; 3]),
}
impl Default for Accent {
    fn default() -> Self {
        Self::Preset(0)
    }
}
/// Nombre y color para tema oscuro y claro.
pub const ACCENTS: [(&str, [u8; 3], [u8; 3]); 8] = [
    ("Violeta", [124, 108, 246], [99, 82, 232]),
    ("Azul", [77, 140, 255], [37, 99, 235]),
    ("Cian", [34, 200, 230], [8, 145, 178]),
    ("Esmeralda", [16, 196, 140], [5, 150, 105]),
    ("Ámbar", [245, 166, 35], [196, 120, 6]),
    ("Naranja", [249, 115, 22], [234, 88, 12]),
    ("Rosa", [236, 84, 160], [219, 39, 119]),
    ("Rojo", [239, 83, 80], [220, 38, 38]),
];
impl Accent {
    pub fn color(self, dark: bool) -> Color32 {
        match self {
            Self::Preset(i) => {
                let (_, d, l) = ACCENTS[i.min(ACCENTS.len() - 1)];
                let [r, g, b] = if dark { d } else { l };
                Color32::from_rgb(r, g, b)
            }
            Self::Custom([r, g, b]) => {
                let c = Color32::from_rgb(r, g, b);
                if dark {
                    c
                } else {
                    c.lerp_to_gamma(Color32::BLACK, 0.12)
                }
            }
        }
    }
    pub fn rgb(self, dark: bool) -> [u8; 3] {
        let c = self.color(dark);
        [c.r(), c.g(), c.b()]
    }
}
/// Acento activo, compartido por todas las ventanas: preset `n` o `CUSTOM | rgb`.
static ACCENT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
const CUSTOM: u32 = 1 << 31;
pub fn accent() -> Accent {
    let v = ACCENT.load(std::sync::atomic::Ordering::Relaxed);
    if v & CUSTOM != 0 {
        Accent::Custom([(v >> 16) as u8, (v >> 8) as u8, v as u8])
    } else {
        Accent::Preset(v as usize)
    }
}
/// Cambia el acento y vuelve a aplicar los estilos de egui que dependen de él.
pub fn set_accent(ctx: &egui::Context, value: Accent) {
    let v = match value {
        Accent::Preset(i) => i.min(ACCENTS.len() - 1) as u32,
        Accent::Custom([r, g, b]) => CUSTOM | (r as u32) << 16 | (g as u32) << 8 | b as u32,
    };
    if ACCENT.swap(v, std::sync::atomic::Ordering::Relaxed) != v {
        apply(ctx);
        ctx.request_repaint();
    }
}

static GLASS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Fondo translúcido para que se vea el desenfoque del compositor.
pub fn glass() -> bool {
    GLASS.load(std::sync::atomic::Ordering::Relaxed)
}
pub fn set_glass(ctx: &egui::Context, on: bool) {
    if GLASS.swap(on, std::sync::atomic::Ordering::Relaxed) != on {
        apply(ctx);
        ctx.request_repaint();
    }
}

/// Logotipo de plane-llama-bench como textura, cargado una sola vez por contexto.
pub fn logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("plane-llama-logo");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/plane-llama-128.png"))
            .ok()?;
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [icon.width as usize, icon.height as usize],
        &icon.rgba,
    );
    let texture = ctx.load_texture("plane-llama-logo", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    Some(texture)
}

thread_local! {
    /// Si el contexto actual ya tiene las familias con peso. `Tokens::get` lo actualiza,
    /// y todo widget lo llama antes de componer texto. Hasta que `install_fonts` surte
    /// efecto (en la pasada siguiente) se usa la familia proporcional para no fallar.
    static WEIGHTS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Familias tipográficas con peso propio; egui no sintetiza negritas.
pub fn medium() -> FontFamily {
    FontFamily::Name("medium".into())
}
pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}
fn weighted(size: f32, family: FontFamily) -> FontId {
    if WEIGHTS.get() {
        FontId::new(size, family)
    } else {
        FontId::proportional(size)
    }
}
pub fn icon_font(size: f32) -> FontId {
    weighted(size, FontFamily::Name("phosphor".into()))
}
pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}
pub fn font_medium(size: f32) -> FontId {
    weighted(size, medium())
}
pub fn font_semibold(size: f32) -> FontId {
    weighted(size, semibold())
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let data = [
        (
            "inter",
            include_bytes!("../../../assets/fonts/inter/Inter-Regular.ttf").as_slice(),
        ),
        (
            "inter-medium",
            include_bytes!("../../../assets/fonts/inter/Inter-Medium.ttf").as_slice(),
        ),
        (
            "inter-semibold",
            include_bytes!("../../../assets/fonts/inter/Inter-SemiBold.ttf").as_slice(),
        ),
        (
            "plex-mono",
            include_bytes!("../../../assets/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf")
                .as_slice(),
        ),
    ];
    for (name, bytes) in data {
        fonts.font_data.insert(
            name.into(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    // Las familias propias heredan los respaldos de egui (iconos, emoji, símbolos).
    // Inter trae glifos propios en el área de uso privado donde Phosphor pone sus
    // iconos, así que los iconos se pintan con la familia «phosphor» (`icon_font`).
    let fallback: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|name| name != "phosphor")
        .collect();
    for (family, first) in [
        (FontFamily::Proportional, "inter"),
        (medium(), "inter-medium"),
        (semibold(), "inter-semibold"),
    ] {
        let mut list = vec![first.to_owned()];
        list.extend(fallback.iter().cloned());
        fonts.families.insert(family, list);
    }
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "plex-mono".into());
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let t = Tokens::of(theme);
        let mut style = (*ctx.style_of(theme)).clone();
        style.text_styles = [
            (egui::TextStyle::Small, font(Tokens::FONT_SM)),
            (egui::TextStyle::Body, font(Tokens::FONT_BODY)),
            (egui::TextStyle::Button, font(Tokens::FONT_BODY)),
            (
                egui::TextStyle::Monospace,
                FontId::monospace(Tokens::FONT_SM + Tokens::BORDER),
            ),
            (egui::TextStyle::Heading, font(Tokens::FONT_LG)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(Tokens::S2, Tokens::S2);
        style.spacing.button_padding = egui::vec2(Tokens::S3, Tokens::S1);
        style.spacing.interact_size.y = Tokens::CONTROL;
        style.spacing.combo_height = Tokens::DIALOG_SM;
        style.spacing.menu_margin = egui::Margin::same(Tokens::S2 as i8);
        style.spacing.window_margin = egui::Margin::same(Tokens::S4 as i8);
        style.spacing.slider_rail_height = Tokens::BAR;
        style.spacing.scroll.bar_width = Tokens::S2;
        style.spacing.scroll.floating = true;
        // Los textos se pueden seleccionar y copiar; las zonas clicables lo desactivan
        // (`widgets::no_select`) para que el texto no les robe el clic o el arrastre.
        style.interaction.selectable_labels = true;
        let v = &mut style.visuals;
        v.dark_mode = theme == egui::Theme::Dark;
        v.override_text_color = Some(t.text);
        v.panel_fill = t.bg;
        v.window_fill = t.surface;
        v.extreme_bg_color = t.sunken;
        v.text_edit_bg_color = Some(t.sunken);
        v.faint_bg_color = t.surface_alt;
        v.code_bg_color = t.surface_alt;
        v.hyperlink_color = t.accent_ink;
        v.window_stroke = Stroke::new(Tokens::BORDER, t.border);
        v.window_corner_radius = Tokens::RADIUS_LG.into();
        v.menu_corner_radius = Tokens::RADIUS_MD.into();
        let shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: t.shadow,
        };
        v.window_shadow = shadow;
        v.popup_shadow = shadow;
        v.selection.bg_fill = t.accent_soft;
        v.selection.stroke = Stroke::new(Tokens::BORDER, t.accent_ink);
        v.warn_fg_color = t.warning;
        v.error_fg_color = t.error;
        v.striped = false;
        v.slider_trailing_fill = true;
        v.handle_shape = egui::style::HandleShape::Circle;
        v.collapsing_header_frame = false;
        v.indent_has_left_vline = false;
        v.widgets.noninteractive.bg_fill = t.surface;
        v.widgets.noninteractive.weak_bg_fill = t.surface;
        v.widgets.noninteractive.bg_stroke = Stroke::new(Tokens::BORDER, t.border);
        v.widgets.noninteractive.fg_stroke = Stroke::new(Tokens::BORDER, t.text);
        v.widgets.noninteractive.corner_radius = Tokens::RADIUS_MD.into();
        for (w, fill, stroke) in [
            (&mut v.widgets.inactive, t.surface_alt, t.border),
            (&mut v.widgets.hovered, t.hover, t.border_strong),
            (&mut v.widgets.active, t.hover, t.accent),
            (&mut v.widgets.open, t.hover, t.border_strong),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = Stroke::new(Tokens::BORDER, stroke);
            w.fg_stroke = Stroke::new(Tokens::BORDER + 0.5, t.text);
            w.corner_radius = Tokens::RADIUS_MD.into();
            w.expansion = 0.;
        }
        v.widgets.inactive.fg_stroke = Stroke::new(Tokens::BORDER + 0.5, t.text_dim);
        ctx.set_style_of(theme, style);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn colours_only_in_theme() {
        fn visit(path: &std::path::Path) {
            for entry in std::fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(&path);
                    continue;
                }
                if path.extension().is_some_and(|e| e == "rs")
                    && path.file_name().is_some_and(|n| n != "theme.rs")
                {
                    let source = std::fs::read_to_string(&path).unwrap();
                    for forbidden in [
                        "Color32::",
                        "Rgba::",
                        "Hsva::",
                        "from_rgb(",
                        "from_rgba(",
                        "hex_color!",
                    ] {
                        assert!(
                            !source.contains(forbidden),
                            "{}: color fuera de theme.rs ({forbidden})",
                            path.display()
                        );
                    }
                }
            }
        }
        visit(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    }
    #[test]
    fn palettes_and_proportions() {
        use super::*;
        let (dark, light) = (Tokens::dark(), Tokens::light());
        assert_ne!(dark.text, light.text);
        assert_ne!(dark.surface, light.surface);
        assert_eq!(Tokens::CONTROL, 30.);
        // El texto principal contrasta con su fondo en ambos temas.
        for t in [dark, light] {
            let luma =
                |c: Color32| 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
            assert!((luma(t.text) - luma(t.surface)).abs() > 150.);
        }
    }
    #[test]
    fn accents_roundtrip_and_contrast() {
        use super::*;
        let ctx = egui::Context::default();
        for a in [
            Accent::Preset(4),
            Accent::Custom([10, 200, 30]),
            Accent::Preset(0),
        ] {
            set_accent(&ctx, a);
            assert_eq!(accent(), a);
            let t = Tokens::of(egui::Theme::Dark);
            assert_eq!(t.accent, a.color(true));
        }
        // El ámbar es claro: el texto encima pasa a oscuro.
        set_accent(&ctx, Accent::Preset(4));
        assert_ne!(Tokens::of(egui::Theme::Dark).on_accent, Color32::WHITE);
        set_accent(&ctx, Accent::default());
        assert_eq!(Tokens::of(egui::Theme::Dark).accent, Tokens::dark().accent);
    }
}
