use iced::border::Radius;
use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow, Theme};

pub const APP: Color = Color::from_rgb(0.102, 0.110, 0.125); // #1A1C20
pub const SURFACE: Color = Color::from_rgb(0.165, 0.180, 0.204); // #2A2E34
pub const SURFACE_ALT: Color = Color::from_rgb(0.173, 0.188, 0.220); // #2C3038
pub const BORDER: Color = Color::from_rgb(0.227, 0.247, 0.282); // #3A3F48
pub const PRIMARY: Color = Color::from_rgb(0.957, 0.961, 0.969); // #F4F5F7
pub const SECONDARY: Color = Color::from_rgb(0.655, 0.678, 0.714); // #A7ADB6
pub const TERTIARY: Color = Color::from_rgb(0.545, 0.569, 0.604); // #8B919A
pub const ACCENT: Color = Color::from_rgb(0.357, 0.424, 1.0); // #5B6CFF
pub const ACCENT_BRIGHT: Color = Color::from_rgb(0.239, 0.545, 1.0); // #3D8BFF
pub const SUCCESS: Color = Color::from_rgb(0.239, 0.863, 0.353); // #3DDC5A
pub const METER_GREEN: Color = Color::from_rgb(0.180, 0.745, 0.290); // #2EBE4A
pub const WARNING: Color = Color::from_rgb(0.941, 0.753, 0.314); // #F0C050
pub const ERROR: Color = Color::from_rgb(0.886, 0.231, 0.196); // #E23B32
pub const BUTTON: Color = Color::from_rgb(0.333, 0.353, 0.392); // #555A64
pub const CONTROL_TRACK: Color = Color::from_rgb(0.290, 0.310, 0.345); // #4A4F58
pub const WARNING_BG: Color = Color::from_rgb(0.239, 0.208, 0.125); // #3D3520
pub const PAIRING_BG: Color = Color::from_rgb(0.239, 0.173, 0.094); // #3D2C18
pub const CONNECTED_BG: Color = Color::from_rgb(0.110, 0.180, 0.133); // #1C2E22
pub const ERROR_BG: Color = Color::from_rgb(0.180, 0.110, 0.110); // #2E1C1C
pub const MON_LINK: Color = Color::from_rgb(0.722, 0.612, 1.0); // #B89CFF
pub const BUS_STRIP_BG: Color = Color::from_rgb(0.227, 0.247, 0.282); // #3A3F48
pub const PURPLE: Color = Color::from_rgb(0.600, 0.302, 0.902); // #994DE6
pub const ELEVATED: Color = Color::from_rgb(0.196, 0.216, 0.247); // #32373F
pub const WARNING_ORANGE: Color = Color::from_rgb(0.878, 0.580, 0.235); // #E0943C

pub const RADIUS_SM: f32 = 2.0;
pub const RADIUS_MD: f32 = 4.0;
pub const RADIUS_LG: f32 = 6.0;
pub const TITLEBAR_HEIGHT: f32 = 32.0;
pub const NAV_HEIGHT: f32 = 52.0;
pub const STRIP_WIDTH: f32 = 108.0;
pub const BUS_STRIP_WIDTH: f32 = 140.0;
pub const FADER_HEIGHT: f32 = 390.0;
pub const METER_HEIGHT: f32 = 40.0;

pub fn flow8_theme() -> iced::Theme {
    iced::Theme::custom(
        "FLOW 8 Dark".to_string(),
        iced::theme::Palette {
            background: APP,
            text: PRIMARY,
            primary: ACCENT,
            success: SUCCESS,
            danger: ERROR,
            warning: WARNING,
        },
    )
}

fn r(radius: f32) -> Radius {
    radius.into()
}

pub fn surface_box(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: Border {
            color: BORDER,
            width: 0.0,
            radius: r(RADIUS_SM),
        },
        ..Default::default()
    }
}

pub fn bus_strip_box(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BUS_STRIP_BG)),
        border: Border {
            radius: r(RADIUS_LG),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn navbar_box(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_ALT)),
        ..Default::default()
    }
}

pub fn app_box(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(APP)),
        ..Default::default()
    }
}

pub fn titlebar_box(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(APP)),
        ..Default::default()
    }
}

pub fn elevated_box(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(ELEVATED)),
        border: Border {
            radius: r(RADIUS_LG),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[derive(Clone, Copy)]
pub enum BtnKind {
    Primary,
    Secondary,
    Quiet,
    Danger,
    Success,
}

pub fn btn(kind: BtnKind) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let base = match kind {
            BtnKind::Primary => ACCENT,
            BtnKind::Secondary => BUTTON,
            BtnKind::Quiet => CONTROL_TRACK,
            BtnKind::Danger => ERROR,
            BtnKind::Success => SUCCESS,
        };
        let background = match status {
            button::Status::Hovered => Color {
                r: (base.r * 1.08).min(1.0),
                g: (base.g * 1.08).min(1.0),
                b: (base.b * 1.08).min(1.0),
                a: base.a,
            },
            button::Status::Pressed => Color {
                r: base.r * 0.92,
                g: base.g * 0.92,
                b: base.b * 0.92,
                a: base.a,
            },
            _ => base,
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: PRIMARY,
            border: Border {
                radius: r(RADIUS_MD),
                ..Default::default()
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

pub fn text_link(_theme: &Theme, status: button::Status) -> button::Style {
    let color = match status {
        button::Status::Hovered => PRIMARY,
        _ => SECONDARY,
    };
    button::Style {
        background: None,
        text_color: color,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}
