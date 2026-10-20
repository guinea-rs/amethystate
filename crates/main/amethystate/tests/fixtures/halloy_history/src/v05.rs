use crate::common::*;
use amethystate::{AmeData, MigrationContext, MigrationResult, ReactiveMap, amethystate, migrate};
#[amethystate]
pub struct Sidebar {
    #[amestate(default = BufferAction::NewPane)]
    pub default_action: BufferAction,
    #[amestate(default = 120)]
    pub width: u16,
}
#[amethystate(prefix = "halloy", version = 5)]
pub struct Config {
    #[amestate(nested)]
    pub font: Font,
    #[amestate(default = {})]
    pub servers: ReactiveMap<String, Server>,
    #[amestate(nested)]
    pub buffer: Buffer,
    #[amestate(nested)]
    pub sidebar: Sidebar,
}
#[migrate(explicit)]
fn migrate_sidebar(
    old: AmeData<crate::v04::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    Ok(AmeData::<Sidebar> {
        default_action: old.default_action,
        width: old.width,
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v04::Config>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        font: old.font,
        servers: old.servers,
        buffer: old.buffer,
        sidebar: _ctx.nested("sidebar", old.dashboard.sidebar)?,
    })
}
#[cfg(not(feature = "s06"))]
pub fn check(store: &Store) {
    let config = Config::new_with(store);
    check_stable(&config.font(), &config.buffer());
    assert_eq!(config.servers().len(), 2);
    assert_eq!(
        config.servers().get("libera.net"),
        Some(Server {
            nickname: "ignat".into(),
            server: "irc.libera.chat".into(),
            channels: vec!["#rust".into(), "#halloy".into()]
        })
    );
    assert_eq!(
        config.servers().get("oftc"),
        Some(Server {
            nickname: "second".into(),
            server: "irc.oftc.net".into(),
            channels: vec![]
        })
    );
    assert_eq!(
        config.sidebar().default_action().get(),
        BufferAction::ReplacePane,
        "sidebar.default_action"
    );
    assert_eq!(config.sidebar().width().get(), 173, "sidebar.width");
}
#[cfg(not(feature = "s06"))]
pub fn exercise(store: &Store, new_schema: bool) {
    if new_schema {
        let config = Config::new_with(store);
        assert_eq!(config.servers().len(), 2);
        assert_eq!(
            config.servers().get("libera.net"),
            Some(Server {
                nickname: "ignat".into(),
                server: "irc.libera.chat".into(),
                channels: vec!["#rust".into(), "#halloy".into()]
            })
        );
        assert_eq!(
            config.servers().get("oftc"),
            Some(Server {
                nickname: "second".into(),
                server: "irc.oftc.net".into(),
                channels: vec![]
            })
        );
        assert_eq!(
            config.sidebar().default_action().get(),
            BufferAction::ReplacePane,
            "sidebar.default_action"
        );
        assert_eq!(config.sidebar().width().get(), 173, "sidebar.width");
    }
    check(store);
}
