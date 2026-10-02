use amethystate::amethystate;

#[amethystate(prefix = "window", mode = "both")]
pub struct Window {
    #[amestate(default = 800u32)]
    pub width: u32,
}

fn main() {}
