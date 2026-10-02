use amethystate::amethystate;

#[amethystate(open = manual)]
pub struct Inner {
    #[amestate(default = "localhost".to_string())]
    pub host: String,
}

#[amethystate(prefix = "agent", open = generated)]
pub struct Agent {
    #[amestate(default = 1u32)]
    pub workers: u32,
}
fn main() {}
