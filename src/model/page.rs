use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    DeviceSelect,
    Mixer,
    Eq,
    Sends,
    Fx,
    Snapshots,
    Settings,
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match *self {
            Page::DeviceSelect => "Device",
            Page::Mixer => "Mixer",
            Page::Eq => "EQ",
            Page::Sends => "Sends",
            Page::Fx => "FX",
            Page::Snapshots => "Snapshots",
            Page::Settings => "Settings",
        };
        write!(f, "{}", text)
    }
}
