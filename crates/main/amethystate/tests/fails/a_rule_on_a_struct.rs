use amethystate::amethystate;
use amethystate::store::RuleContext;

fn the_window_can_be_drawn(window: &mut amethystate::AmeData<Window>, _cx: &RuleContext) {
    window.max = window.max.max(window.min);
}

#[amethystate(prefix = "window", rule = the_window_can_be_drawn)]
pub struct Window {
    #[amestate(default = 400u32)]
    pub min: u32,

    #[amestate(default = 1600u32)]
    pub max: u32,
}

fn main() {}
