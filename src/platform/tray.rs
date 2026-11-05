pub struct TrayConfig {
    pub close_to_tray: bool,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            close_to_tray: false,
        }
    }
}

#[cfg(feature = "tray")]
pub fn create_tray_menu() -> Result<(), String> {
    use tray_icon::{
        menu::{Menu, MenuItem},
        TrayIconBuilder,
    };

    let menu = Menu::new();
    let show_item = MenuItem::new("Show/Hide", true, None);
    let quit_item = MenuItem::new("Exit", true, None);
    menu.append(&show_item)
        .map_err(|e| format!("Menu: {}", e))?;
    menu.append(&quit_item)
        .map_err(|e| format!("Menu: {}", e))?;

    let icon = load_tray_icon();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("FLOW 8 MIDI Controller")
        .with_icon(icon)
        .build()
        .map_err(|e| format!("Tray: {}", e))?;

    Ok(())
}

#[cfg(feature = "tray")]
fn load_tray_icon() -> tray_icon::Icon {
    let icon_bytes = include_bytes!("../../resources/icon.ico");
    let img = image::load_from_memory(icon_bytes).expect("Failed to load tray icon");
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    tray_icon::Icon::from_rgba(rgba.into_raw(), w, h).expect("Failed to create tray icon")
}

#[cfg(not(feature = "tray"))]
pub fn create_tray_menu() -> Result<(), String> {
    Ok(())
}
