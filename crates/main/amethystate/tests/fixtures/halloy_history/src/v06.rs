use crate::common::*;
use amethystate::{
    AmeData, MigrationContext, MigrationResult, ReactiveMap, amethystate, migrate, migrate_field,
};
#[amethystate]
pub struct Buttons {
    #[amestate(default = true)]
    pub file_transfer: bool,
    #[amestate(default = true)]
    pub command_bar: bool,
}
#[amethystate]
pub struct Sidebar {
    #[amestate(default = BufferAction::NewPane)]
    pub default_action: BufferAction,
    #[amestate(default = 120)]
    pub width: u16,
    #[amestate(nested)]
    pub buttons: Buttons,
}
#[amethystate(prefix = "halloy", version = 6)]
pub struct Config {
    #[amestate(nested)]
    pub font: Font,
    #[amestate(default = {})]
    pub servers: ReactiveMap<String, Server>,
    #[amestate(nested)]
    pub buffer: Buffer,
    #[amestate(nested)]
    pub sidebar: Sidebar,
    #[amestate(default = true)]
    pub tooltips: bool,
}
#[migrate(explicit)]
fn migrate_sidebar(
    old: AmeData<crate::v05::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    Ok(AmeData::<Sidebar> {
        default_action: old.default_action,
        width: old.width,
        buttons: AmeData::<Buttons> {
            file_transfer: true,
            command_bar: true,
        },
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v05::Config>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        font: old.font,
        servers: old.servers,
        buffer: old.buffer,
        sidebar: migrate_field!(_ctx, old.sidebar),
        tooltips: true,
    })
}
#[cfg(not(feature = "s07"))]
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
    assert!(
        !config.sidebar().buttons().file_transfer().get(),
        "sidebar.buttons.file_transfer"
    );
    assert!(
        !config.sidebar().buttons().command_bar().get(),
        "sidebar.buttons.command_bar"
    );
    assert!(!config.tooltips().get(), "tooltips");
}
#[cfg(not(feature = "s07"))]
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
        assert!(
            config.sidebar().buttons().file_transfer().get(),
            "sidebar.buttons.file_transfer"
        );
        assert!(
            config.sidebar().buttons().command_bar().get(),
            "sidebar.buttons.command_bar"
        );
        assert!(config.tooltips().get(), "tooltips");
        config.sidebar().buttons().file_transfer().set(false);
        config.sidebar().buttons().command_bar().set(false);
        config.tooltips().set(false);
    }
    check(store);
}
