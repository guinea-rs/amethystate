use amethystate::amethystate;
use amethystate::store::{Invalid, RuleContext};

fn a_reachable_host(inner: &mut Inner, _cx: &RuleContext) -> Result<(), Invalid> {
    if inner.host().get().is_empty() {
        Err(Invalid::new("a host nobody can reach"))
    } else {
        Ok(())
    }
}

#[amethystate]
pub struct Inner {
    #[amestate(default = "localhost".to_string())]
    pub host: String,
}

#[amethystate(prefix = "cfg")]
pub struct Cfg {
    #[amestate(nested, rule = a_reachable_host)]
    pub net: Inner,
}

fn main() {}
