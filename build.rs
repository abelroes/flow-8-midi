fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        winresource::WindowsResource::new()
            .set_icon("./resources/icon.ico")
            .compile()
            .expect("Failed to set Windows icon");
    }
}
