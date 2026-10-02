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

#[amethystate]
pub struct Sizes {
    #[amestate(path = "window.width", default = 640u32)]
    pub width: u32,
}

#[amethystate(prefix = "ui")]
pub struct Ui {
    #[amestate(nested, flatten)]
    pub layout: Layout,

    #[amestate(nested, flatten)]
    pub sizes: Sizes,
}

fn main() {}
