use amethystate::amethystate;
use amethystate::store::RuleContext;

fn a_reachable_host(inner: &mut Inner, _cx: &RuleContext) {
    if inner.host().get().is_empty() {
        inner.host().set("localhost".to_string());
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
