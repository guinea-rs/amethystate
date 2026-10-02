use amethystate::amethystate;

#[amethystate]
pub struct Window {
    #[amestate(path = "window.width", default = 800u32)]
    pub width: u32,
}

#[amethystate(prefix = "ui")]
pub struct Ui {
    #[amestate(nested, flatten)]
    pub layout: Window,

    #[amestate(default = 0u32)]
    pub window: u32,
}

fn main() {}
