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
    #[amestate(default = true)]
    pub reload_config: bool,
    #[amestate(default = true)]
    pub theme_editor: bool,
}
#[amethystate]
pub struct Sidebar {
    #[amestate(default = BufferAction::NewPane)]
    pub buffer_action: BufferAction,
    #[amestate(default = None)]
    pub buffer_focused_action: Option<FocusedAction>,
    #[amestate(default = 120)]
    pub width: u16,
    #[amestate(nested)]
    pub buttons: Buttons,
    #[amestate(default = UnreadChoice::Dot)]
    pub unread_indicator: UnreadChoice,
    #[amestate(default = Position::Left)]
    pub position: Position,
}
#[amethystate(prefix = "halloy", version = 10)]
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
    old: AmeData<crate::v09::Buttons>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Buttons>> {
    Ok(AmeData::<Buttons> {
        file_transfer: old.file_transfer,
        command_bar: old.command_bar,
        reload_config: old.reload_config,
        theme_editor: true,
    })
}
#[migrate(explicit)]
fn migrate_sidebar(
    old: AmeData<crate::v09::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    Ok(AmeData::<Sidebar> {
        buffer_action: old.buffer_action,
        buffer_focused_action: old.buffer_focused_action,
        width: old.width,
        buttons: migrate_field!(_ctx, old.buttons),
        unread_indicator: old.unread_indicator,
        position: old.position,
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v09::Config>,
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
#[cfg(not(feature = "s11"))]
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
        config.sidebar().buffer_action().get(),
        BufferAction::ReplacePane,
        "sidebar.buffer_action"
    );
    assert_eq!(
        config.sidebar().buffer_focused_action().get(),
        Some(FocusedAction::ClosePane),
        "sidebar.buffer_focused_action"
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
        !config.sidebar().buttons().reload_config().get(),
        "sidebar.buttons.reload_config"
    );
    assert!(
        !config.sidebar().buttons().theme_editor().get(),
        "sidebar.buttons.theme_editor"
    );
    assert_eq!(
        config.sidebar().unread_indicator().get(),
        UnreadChoice::Title,
        "sidebar.unread_indicator"
    );
    assert_eq!(
        config.sidebar().position().get(),
        Position::Right,
        "sidebar.position"
    );
    assert!(!config.tooltips().get(), "tooltips");
}
#[cfg(not(feature = "s11"))]
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
            config.sidebar().buffer_action().get(),
            BufferAction::ReplacePane,
            "sidebar.buffer_action"
        );
        assert_eq!(
            config.sidebar().buffer_focused_action().get(),
            Some(FocusedAction::ClosePane),
            "sidebar.buffer_focused_action"
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
            !config.sidebar().buttons().reload_config().get(),
            "sidebar.buttons.reload_config"
        );
        assert!(
            config.sidebar().buttons().theme_editor().get(),
            "sidebar.buttons.theme_editor"
        );
        assert_eq!(
            config.sidebar().unread_indicator().get(),
            UnreadChoice::Title,
            "sidebar.unread_indicator"
        );
        assert_eq!(
            config.sidebar().position().get(),
            Position::Right,
            "sidebar.position"
        );
        assert!(!config.tooltips().get(), "tooltips");
        config.sidebar().buttons().theme_editor().set(false);
    }
    check(store);
}
