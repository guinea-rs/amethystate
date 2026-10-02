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
}

#[amethystate(prefix = "ui")]
pub struct Ui {
    #[amestate(nested, flatten)]
    pub layout: Layout,

    #[amestate(path = "window.width", default = 0u32)]
    pub width: u32,
}

fn main() {}
