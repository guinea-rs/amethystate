use amethystate::store::OpenStruct;
use amethystate::{Open, Schema, Store, amethystate};

#[amethystate(prefix = "agent", open = manual)]
pub struct Agent {
    #[amestate(default = 1u32)]
    pub workers: u32,
}

impl Open for Agent {
    fn new_with(store: &Store) -> Result<Self, OpenStruct> {
        <Self as Schema>::open(store)
    }
}

#[amethystate(prefix = "kept_agent", mode = "persistent", open = manual)]
pub struct KeptAgent {
    #[amestate(default = 1u32)]
    pub workers: u32,
}

impl Open for KeptAgent {
    fn new_with(store: &Store) -> Result<Self, OpenStruct> {
        <Self as Schema>::open(store)
    }
}

fn main() {
    let _ = Agent::new();
    let _ = KeptAgent::load();
}
