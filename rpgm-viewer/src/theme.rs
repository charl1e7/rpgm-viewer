use egui::{Color32, CornerRadius, Margin, Shadow, Stroke, Vec2, style::ScrollStyle};

struct Palette {
    bg_extreme: Color32,
    bg_panel: Color32,
    bg_window: Color32,
    bg_widget: Color32,
    bg_widget_hover: Color32,
    bg_widget_active: Color32,
    accent: Color32,
    accent_bright: Color32,
    text: Color32,
    text_dim: Color32,
    stroke_faint: Color32,
    warn: Color32,
    error: Color32,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            bg_extreme: Color32::from_rgb(0x0F, 0x0C, 0x18),
            bg_panel: Color32::from_rgb(0x18, 0x14, 0x27),
            bg_window: Color32::from_rgb(0x1D, 0x18, 0x30),
            bg_widget: Color32::from_rgb(0x27, 0x20, 0x3F),
            bg_widget_hover: Color32::from_rgb(0x35, 0x2A, 0x57),
            bg_widget_active: Color32::from_rgb(0x46, 0x35, 0x78),
            accent: Color32::from_rgb(0xA6, 0x7C, 0xFA),
            accent_bright: Color32::from_rgb(0xC8, 0xAC, 0xFF),
            text: Color32::from_rgb(0xEC, 0xE9, 0xF7),
            text_dim: Color32::from_rgb(0xA6, 0x9F, 0xC2),
            stroke_faint: Color32::from_rgb(0x39, 0x30, 0x57),
            warn: Color32::from_rgb(0xF2, 0xC1, 0x4E),
            error: Color32::from_rgb(0xFF, 0x6B, 0x8B),
        }
    }
}

fn visuals() -> egui::Visuals {
    let p = Palette::default();
    let mut v = egui::Visuals::dark();

    v.override_text_color = None;
    v.panel_fill = p.bg_panel;
    v.window_fill = p.bg_window;
    v.window_stroke = Stroke::new(1.0, p.stroke_faint);
    v.extreme_bg_color = p.bg_extreme;
    v.faint_bg_color = p.bg_widget;
    v.code_bg_color = p.bg_extreme;
    v.hyperlink_color = p.accent_bright;
    v.warn_fg_color = p.warn;
    v.error_fg_color = p.error;
    
    v.text_edit_bg_color = Some(p.bg_extreme);

    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(10);
    
    v.window_shadow = Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(0x1D, 0x18, 0x30, 180),
    };
    v.popup_shadow = v.window_shadow;

    v.selection.bg_fill = p.accent.linear_multiply(0.55);
    v.selection.stroke = Stroke::new(1.0, p.accent_bright);

    v.widgets.noninteractive.bg_fill = p.bg_panel;
    v.widgets.noninteractive.weak_bg_fill = p.bg_panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.stroke_faint);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text_dim);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(10);

    v.widgets.inactive.bg_fill = p.bg_widget;
    v.widgets.inactive.weak_bg_fill = p.bg_widget;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.stroke_faint);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.corner_radius = CornerRadius::same(9);
    v.widgets.inactive.expansion = 0.0;

    v.widgets.hovered.bg_fill = p.bg_widget_hover;
    v.widgets.hovered.weak_bg_fill = p.bg_widget_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent); 
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, p.accent_bright);
    v.widgets.hovered.corner_radius = CornerRadius::same(9);
    v.widgets.hovered.expansion = 0.0; 

    v.widgets.active.bg_fill = p.bg_widget_active;
    v.widgets.active.weak_bg_fill = p.bg_widget_active;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent_bright); 
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(9);
    v.widgets.active.expansion = 0.0; 

    v.widgets.open.bg_fill = p.bg_widget_active;
    v.widgets.open.weak_bg_fill = p.bg_widget_active;
    v.widgets.open.bg_stroke = Stroke::new(1.0, p.accent); 
    v.widgets.open.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.open.corner_radius = CornerRadius::same(9);
    v.widgets.open.expansion = 0.0; 

    v.indent_has_left_vline = false;
    v.striped = true;
    v.slider_trailing_fill = true;
    v.collapsing_header_frame = true;
    v.button_frame = true;
    v.resize_corner_size = 10.0;

    v
}

pub fn apply(ctx: &egui::Context) {
    let visuals = visuals();

    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();

        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(10.0, 6.0);
        style.spacing.window_margin = Margin::same(14);
        style.spacing.menu_margin = Margin::same(8);
        style.spacing.indent = 18.0;
        style.spacing.icon_spacing = 8.0;

        style.spacing.scroll = ScrollStyle::thin();
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.floating_allocated_width = 4.0;
        style.spacing.scroll.dormant_background_opacity = 0.3;
        style.spacing.scroll.dormant_handle_opacity = 0.5;

        style.animation_time = 0.12;
    });
}