use crate::common::*;
use amethystate::{AmeData, MigrationContext, MigrationResult, ReactiveMap, amethystate, migrate};
#[amethystate]
pub struct Dashboard {
    #[amestate(default = BufferAction::NewPane)]
    pub sidebar_default_action: BufferAction,
}
#[amethystate(prefix = "halloy", version = 2)]
pub struct Config {
    #[amestate(nested)]
    pub font: Font,
    #[amestate(default = {})]
    pub servers: ReactiveMap<String, Server>,
    #[amestate(nested)]
    pub new_buffer: Buffer,
    #[amestate(nested)]
    pub dashboard: Dashboard,
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v01::Config>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        font: old.font,
        servers: old.servers,
        new_buffer: old.new_buffer,
        dashboard: AmeData::<Dashboard> {
            sidebar_default_action: BufferAction::NewPane,
        },
    })
}
#[cfg(not(feature = "s03"))]
pub fn check(store: &Store) {
    let config = Config::new_with(store);
    check_stable(&config.font(), &config.new_buffer());
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
        config.dashboard().sidebar_default_action().get(),
        BufferAction::ReplacePane,
        "dashboard.sidebar_default_action"
    );
}
#[cfg(not(feature = "s03"))]
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
            config.dashboard().sidebar_default_action().get(),
            BufferAction::NewPane,
            "dashboard.sidebar_default_action"
        );
        config
            .dashboard()
            .sidebar_default_action()
            .set(BufferAction::ReplacePane);
    }
    check(store);
}
