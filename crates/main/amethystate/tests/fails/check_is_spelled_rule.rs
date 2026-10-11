use amethystate::amethystate;
use amethystate::store::RuleContext;

fn a_port_that_is_not_zero(port: &mut u16, _cx: &RuleContext) {
    if *port == 0 {
        *port = 8080;
    }
}

#[amethystate(prefix = "cfg")]
pub struct Cfg {
    #[amestate(default = 8080u16, check = a_port_that_is_not_zero)]
    pub port: u16,
}

fn main() {}
