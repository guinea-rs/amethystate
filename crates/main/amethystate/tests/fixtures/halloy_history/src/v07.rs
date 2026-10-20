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
    #[amestate(default = true)]
    pub show_unread_indicators: bool,
}
#[amethystate(prefix = "halloy", version = 7)]
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
fn migrate_buttons(
    old: AmeData<crate::v06::Buttons>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Buttons>> {
    Ok(AmeData::<Buttons> {
        file_transfer: old.file_transfer,
        command_bar: old.command_bar,
    })
}
#[migrate(explicit)]
fn migrate_sidebar(
    old: AmeData<crate::v06::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    Ok(AmeData::<Sidebar> {
        default_action: old.default_action,
        width: old.width,
        buttons: migrate_field!(_ctx, old.buttons),
        show_unread_indicators: true,
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v06::Config>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        font: old.font,
        servers: old.servers,
        buffer: old.buffer,
        sidebar: migrate_field!(_ctx, old.sidebar),
        tooltips: old.tooltips,
    })
}
#[cfg(not(feature = "s08"))]
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
    assert!(
        !config.sidebar().show_unread_indicators().get(),
        "sidebar.show_unread_indicators"
    );
    assert!(!config.tooltips().get(), "tooltips");
}
#[cfg(not(feature = "s08"))]
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
            !config.sidebar().buttons().file_transfer().get(),
            "sidebar.buttons.file_transfer"
        );
        assert!(
            !config.sidebar().buttons().command_bar().get(),
            "sidebar.buttons.command_bar"
        );
        assert!(
            config.sidebar().show_unread_indicators().get(),
            "sidebar.show_unread_indicators"
        );
        assert!(!config.tooltips().get(), "tooltips");
        config.sidebar().show_unread_indicators().set(false);
    }
    check(store);
}
