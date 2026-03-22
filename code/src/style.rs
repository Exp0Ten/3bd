use iced::{
    Theme, Color, Background, Border,
    border::Radius,
    widget::{
        container, button, svg, text, text_input
    }
};


/// FILE: style.rs - Styling functions for the ui.rs

// Container styles
pub fn back(theme: &Theme) -> container::Style {
    let pallete = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(pallete.background.base.color)),
        border: Border {
            radius: Radius::new(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn bar(theme: &Theme) -> container::Style {
    let pallete = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(pallete.background.weak.color)),
        border: Border {
            radius: Radius::new(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

// Text styles
pub fn error(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.extended_palette().danger.base.color) }
}

pub fn weak(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.extended_palette().background.weak.color) }
}

pub fn widget_text(theme: &Theme) -> text::Style {
    let pallete = theme.extended_palette();
    text::Style { color: Some(pallete.primary.base.color) }
}

pub fn widget_text_toggled(theme: &Theme) -> text::Style {
    let pallete = theme.extended_palette();
    text::Style { color: Some(pallete.background.base.color) }
}

// pane titlebar style
pub fn pane_title(theme: &Theme) -> container::Style {
    let pallete = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(color_darken(pallete.background.weak.color, 0.2))),
        border: Border {
            radius: Radius::new(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

// SVG styles
pub fn bar_svg(theme: &Theme, _status: svg::Status) -> svg::Style {
    let pallete = theme.extended_palette();
    svg::Style { color: Some(pallete.background.base.text) }
}

pub fn bar_svg_toggled(theme: &Theme, _status: svg::Status) -> svg::Style {
    let pallete = theme.extended_palette();
    svg::Style { color: Some(pallete.background.base.color) }
}

pub fn widget_svg(theme: &Theme, _status: svg::Status) -> svg::Style {
    let pallete = theme.extended_palette();
    svg::Style { color: Some(pallete.primary.base.color) }
}

pub fn button_svg_disabled(theme: &Theme, _status: svg::Status) -> svg::Style {
    let pallete = theme.extended_palette();
    svg::Style { color: Some(pallete.primary.weak.color) }
}

pub fn widget_svg_toggled(theme: &Theme, _status: svg::Status) -> svg::Style {
    let pallete = theme.extended_palette();
    svg::Style { color: Some(pallete.background.base.color) }
}

// lighter on hover, darker on press (as a normal button but needs to be specified with custom style), and toggled versions of the buttons
pub fn bar_button(theme: &Theme, status: button::Status) -> button::Style {
    let pallete = theme.extended_palette();
    let color = Color::TRANSPARENT;
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(color_mix(color, pallete.background.base.text, 0.6))),
            button::Status::Pressed => Some(Background::Color(color_darken(color, 0.3))),
            _ => Some(Background::Color(color))
        },
        border: Border {
            radius: Radius::new(5),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn bar_button_toggled(theme: &Theme, status: button::Status) -> button::Style {
    let pallete = theme.extended_palette();
    let color = pallete.background.base.text;
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(color_lighten(color, 0.3))),
            button::Status::Pressed => Some(Background::Color(color_darken(color, 0.3))),
            _ => Some(Background::Color(color))
        },
        border: Border {
            radius: Radius::new(5),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn widget_button(theme: &Theme, status: button::Status) -> button::Style {
    let pallete = theme.extended_palette();
    let color = Color::TRANSPARENT;
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(color_mix(color, pallete.background.base.text, 0.6))),
            button::Status::Pressed => Some(Background::Color(color_darken(color, 0.3))),
            _ => Some(Background::Color(color))
        },
        border: Border {
            radius: Radius::new(5),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn widget_button_toggled(theme: &Theme, status: button::Status) -> button::Style {
    let pallete = theme.extended_palette();
    let color = pallete.primary.base.color;
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(color_lighten(color, 0.3))),
            button::Status::Pressed => Some(Background::Color(color_darken(color, 0.3))),
            _ => Some(Background::Color(color))
        },
        border: Border {
            radius: Radius::new(5),
            ..Default::default()
        },
        ..Default::default()
    }
}

// invisible button, reg dot on hover, shown when toggled
pub fn breakpoint(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        ..Default::default()
    }
}

pub fn breakpoint_svg(theme: &Theme, status: svg::Status) -> svg::Style {
    svg::Style {
        color: match status {
            svg::Status::Idle => Some(theme.extended_palette().background.base.color),
            svg::Status::Hovered => Some(theme.extended_palette().danger.weak.color) // red, i mean maybe ill change it to some theme like color
        }
    }
}

pub fn breakpoint_svg_toggled(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(theme.extended_palette().danger.base.color)
    }
}

// just like breakpoint, but with different color
pub fn collapse_svg(theme: &Theme, status: svg::Status) -> svg::Style {
    svg::Style {
        color: match status {
            svg::Status::Idle => Some(theme.extended_palette().background.base.color),
            svg::Status::Hovered => Some(theme.extended_palette().secondary.base.color)
        }
    }
}

pub fn collapse_svg_toggled(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(theme.extended_palette().secondary.base.color)
    }
}

// other styles
pub fn address(theme: &Theme, status: text_input::Status, incorrect: bool) -> text_input::Style {
    let mut default = text_input::default(theme, status);
    default.value = theme.extended_palette().primary.base.color;
    default.selection = theme.extended_palette().background.weak.color;
    if incorrect {
        default.value = theme.extended_palette().danger.base.color;
        default.border.color = theme.extended_palette().danger.base.color;
    }
    default
}

pub fn line(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.extended_palette().primary.base.color) }
}

pub fn terminal(theme: &Theme) -> container::Style {
    let pallete = theme.extended_palette();
    container::Style {
        border: Border {
            radius: Radius::new(5),
            color: pallete.secondary.base.color,
            width: 2.
        },
        ..Default::default()
    }
}


// Color Helpers

fn color_mix(color_a: Color, color_b: Color, factor: f32) -> Color { // averaging the color vector
    Color {
        r: color_b.r * factor + color_a.r * (1.-factor),
        g: color_b.g * factor + color_a.g * (1.-factor),
        b: color_b.b * factor + color_a.b * (1.-factor),
        a: color_b.a * factor + color_a.a * (1.-factor)
    }
}

fn color_lighten(color: Color, factor: f32) -> Color { // mix with white
    color_mix(color, Color::WHITE, factor)
}

fn color_darken(color: Color, factor: f32) -> Color { // mix with black
    color_mix(color, Color::BLACK, factor)
}
