use amethystate::amethystate;

#[amethystate]
pub struct Window {
    #[amestate(default = 800u32)]
    pub width: u32,
}

#[amethystate]
pub struct Layout {
    #[amestate(nested)]
    pub window: Window,

    #[amestate(path = "panel.width", default = 240u32)]
    pub panel: u32,
}

#[amethystate(prefix = "ui")]
pub struct Ui {
    #[amestate(nested, flatten)]
    pub layout: Layout,

    #[amestate(path = "sidebar.width", default = 200u32)]
    pub sidebar: u32,

    #[amestate(path = "windows", default = 1u32)]
    pub windows: u32,
}

fn main() {}
