use crate::common::*;
use amethystate::{
    AmeData, MigrationContext, MigrationResult, ReactiveMap, amethystate, migrate, migrate_field,
};
#[amethystate]
pub struct Sidebar {
    #[amestate(default = BufferAction::NewPane)]
    pub buffer_action: BufferAction,
    #[amestate(default = None)]
    pub buffer_focused_action: Option<FocusedAction>,
    #[amestate(default = None)]
    pub max_width: Option<u16>,
    #[amestate(default = UnreadChoice::Dot)]
    pub unread_indicator: UnreadChoice,
    #[amestate(default = Position::Left)]
    pub position: Position,
    #[amestate(default = true)]
    pub show_user_menu: bool,
}
#[amethystate(prefix = "halloy", version = 12)]
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
#[rename(width => max_width)]
fn migrate_sidebar(
    old: AmeData<crate::v11::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    Ok(AmeData::<Sidebar> {
        buffer_action: old.buffer_action,
        buffer_focused_action: old.buffer_focused_action,
        max_width: Some(old.width),
        unread_indicator: old.unread_indicator,
        position: old.position,
        show_user_menu: old.show_user_menu,
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v11::Config>,
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
#[cfg(not(feature = "s13"))]
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
    assert_eq!(
        config.sidebar().max_width().get(),
        Some(173),
        "sidebar.max_width"
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
    assert!(
        !config.sidebar().show_user_menu().get(),
        "sidebar.show_user_menu"
    );
    assert!(!config.tooltips().get(), "tooltips");
}
#[cfg(not(feature = "s13"))]
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
        assert_eq!(
            config.sidebar().max_width().get(),
            Some(173),
            "sidebar.max_width"
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
        assert!(
            !config.sidebar().show_user_menu().get(),
            "sidebar.show_user_menu"
        );
        assert!(!config.tooltips().get(), "tooltips");
    }
    check(store);
}
