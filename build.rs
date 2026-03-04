#[cfg(windows)]
use {std::io, winresource::WindowsResource};

fn main() {
    #[cfg(windows)]
    set_windows_icon().expect("Failed to set Windows icon");
}

#[cfg(windows)]
fn set_windows_icon() -> io::Result<()> {
    WindowsResource::new()
        .set_icon("./resources/icon.ico")
        .compile()
}
