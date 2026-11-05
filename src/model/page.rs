use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Connection,
    Home,
    Mixer,
    Stage,
    Eq,
    Sends,
    Fx,
    Snapshots,
    Setup,

    // Legacy alias
    DeviceSelect,
    Settings,
}

impl Page {
    pub fn is_main_section(self) -> bool {
        matches!(
            self,
            Page::Mixer | Page::Stage | Page::Eq | Page::Sends | Page::Fx
        )
    }

    pub fn needs_navbar(self) -> bool {
        !matches!(self, Page::Connection | Page::DeviceSelect)
    }
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match *self {
            Page::Connection | Page::DeviceSelect => "Connection",
            Page::Home => "Home",
            Page::Mixer => "Mixer",
            Page::Stage => "Stage",
            Page::Eq => "EQ",
            Page::Sends => "Sends",
            Page::Fx => "FX",
            Page::Snapshots => "Snapshots",
            Page::Setup | Page::Settings => "Setup",
        };
        write!(f, "{}", text)
    }
}
