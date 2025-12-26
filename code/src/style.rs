use iced::{
    Theme, Color, Background, Border,
    border::Radius,
    widget::{
        container, button, svg
    },

};


pub fn bar(theme: &Theme) -> container::Style {
    let pallete = theme.extended_palette();
    container::Style {
        //text_color: Some(pallete.primary.base.text),
        background: Some(Background::Color(pallete.background.weak.color)),
        border: Border {
            radius: Radius::new(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn bar_svg(theme: &Theme, _status: svg::Status) -> svg::Style {
    let pallete = theme.extended_palette();
    svg::Style { color: Some(pallete.background.weak.text) }
}

pub fn bar_svg_toggled(theme: &Theme, _status: svg::Status) -> svg::Style {
    let pallete = theme.extended_palette();
    svg::Style { color: Some(pallete.background.weak.color) }
}


pub fn bar_button(theme: &Theme, status: button::Status) -> button::Style {
    let _pallete = theme.extended_palette();
    let color = Color::TRANSPARENT;
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(color_lighten(color, 0.6))),
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
    let color = pallete.background.weak.text;
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(color_lighten(color, 0.6))),
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

fn color_mix(color_a: Color, color_b: Color, factor: f32) -> Color {
    Color {
        r: color_b.r * factor + color_a.r * (1.-factor),
        g: color_b.g * factor + color_a.g * (1.-factor),
        b: color_b.b * factor + color_a.b * (1.-factor),
        a: color_b.a * factor + color_a.a * (1.-factor)
    }
}

fn color_lighten(color: Color, factor: f32) -> Color {
    color_mix(color, Color::WHITE, factor)
}

fn color_darken(color: Color, factor: f32) -> Color {
    color_mix(color, Color::BLACK, factor)
}
