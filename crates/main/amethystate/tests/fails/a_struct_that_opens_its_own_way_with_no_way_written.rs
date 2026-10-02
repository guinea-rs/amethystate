use amethystate::amethystate;

#[amethystate(prefix = "agent", open = manual)]
pub struct Agent {
    #[amestate(default = 1u32)]
    pub workers: u32,
}

fn main() {}
