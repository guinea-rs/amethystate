use amethystate::amethystate;
use amethystate::store::{Invalid, RuleContext};

fn the_window_can_be_drawn(
    window: &amethystate::AmeData<Window>,
    _cx: &RuleContext,
) -> Result<(), Invalid> {
    if window.min <= window.max {
        Ok(())
    } else {
        Err(Invalid::new("the smallest window is wider than the largest"))
    }
}

#[amethystate(prefix = "window", rule = the_window_can_be_drawn)]
pub struct Window {
    #[amestate(default = 400u32)]
    pub min: u32,

    #[amestate(default = 1600u32)]
    pub max: u32,
}

fn main() {}
