use amethystate::store::{Invalid, RuleContext};
use amethystate_macros::amethystate;

fn a_width_on_screen(width: &mut u32, _: &RuleContext) -> Result<(), Invalid> {
    *width = (*width).clamp(320, 7680);
    Ok(())
}

#[amethystate(prefix = "editor", target = "tauri-wasm")]
pub struct Editor {
    #[amestate(default = 1280, rule = a_width_on_screen)]
    pub width: u32,

    #[amestate(default = 640, volatile, rule = a_width_on_screen)]
    pub dragged_to: u32,
}

fn main() {}
