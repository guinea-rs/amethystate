use crate::common::*;
use amethystate::{
    AmeData, MigrationContext, MigrationResult, ReactiveMap, amethystate, migrate, migrate_field,
};
#[amethystate]
pub struct Dashboard {
    #[amestate(nested)]
    pub sidebar: Sidebar,
}
#[amethystate]
pub struct Sidebar {
    #[amestate(default = BufferAction::NewPane)]
    pub default_action: BufferAction,
    #[amestate(default = 120)]
    pub width: u16,
}
#[amethystate(prefix = "halloy", version = 4)]
pub struct Config {
    #[amestate(nested)]
    pub font: Font,
    #[amestate(default = {})]
    pub servers: ReactiveMap<String, Server>,
    #[amestate(nested)]
    pub buffer: Buffer,
    #[amestate(nested)]
    pub dashboard: Dashboard,
}
#[migrate(explicit)]
fn migrate_dashboard(
    old: AmeData<crate::v03::Dashboard>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Dashboard>> {
    Ok(AmeData::<Dashboard> {
        sidebar: AmeData::<Sidebar> {
            default_action: old.sidebar_default_action,
            width: 120,
        },
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v03::Config>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        font: old.font,
        servers: old.servers,
        buffer: old.buffer,
        dashboard: migrate_field!(_ctx, old.dashboard),
    })
}
#[cfg(not(feature = "s05"))]
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
        config.dashboard().sidebar().default_action().get(),
        BufferAction::ReplacePane,
        "dashboard.sidebar.default_action"
    );
    assert_eq!(
        config.dashboard().sidebar().width().get(),
        173,
        "dashboard.sidebar.width"
    );
}
#[cfg(not(feature = "s05"))]
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
            config.dashboard().sidebar().default_action().get(),
            BufferAction::ReplacePane,
            "dashboard.sidebar.default_action"
        );
        assert_eq!(
            config.dashboard().sidebar().width().get(),
            120,
            "dashboard.sidebar.width"
        );
        config.dashboard().sidebar().width().set(173);
    }
    check(store);
}
