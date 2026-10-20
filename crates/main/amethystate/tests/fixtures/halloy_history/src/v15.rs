use crate::common::*;
use amethystate::{
    AmeData, MigrationContext, MigrationResult, ReactiveMap, amethystate, migrate, migrate_field,
};
#[amethystate]
pub struct Scrollbar {
    #[amestate(default = 5)]
    pub width: u32,
    #[amestate(default = 5)]
    pub scroller_width: u32,
}
#[amethystate]
pub struct Sidebar {
    #[amestate(default = None)]
    pub max_width: Option<u16>,
    #[amestate(default = UnreadChoice::Dot)]
    pub unread_indicator: UnreadChoice,
    #[amestate(default = Position::Left)]
    pub position: Position,
    #[amestate(default = true)]
    pub show_user_menu: bool,
    #[amestate(default = OrderBy::Alpha)]
    pub order_by: OrderBy,
    #[amestate(nested)]
    pub scrollbar: Scrollbar,
}
#[amethystate]
pub struct Actions {
    #[amestate(nested)]
    pub sidebar: SidebarActions,
}
#[amethystate]
pub struct SidebarActions {
    #[amestate(default = BufferAction::NewPane)]
    pub buffer: BufferAction,
    #[amestate(default = None)]
    pub focused_buffer: Option<FocusedAction>,
}
#[amethystate(prefix = "halloy", version = 15)]
pub struct Config {
    #[amestate(nested)]
    pub font: Font,
    #[amestate(default = {})]
    pub servers: ReactiveMap<String, Server>,
    #[amestate(nested)]
    pub buffer: Buffer,
    #[amestate(nested)]
    pub sidebar: Sidebar,
    #[amestate(nested)]
    pub actions: Actions,
    #[amestate(default = true)]
    pub tooltips: bool,
}
#[migrate(explicit)]
fn migrate_sidebar(
    old: AmeData<crate::v14::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    Ok(AmeData::<Sidebar> {
        max_width: old.max_width,
        unread_indicator: old.unread_indicator,
        position: old.position,
        show_user_menu: old.show_user_menu,
        order_by: old.order_by,
        scrollbar: AmeData::<Scrollbar> {
            width: 5,
            scroller_width: 5,
        },
    })
}
#[migrate(explicit)]
fn migrate_actions(
    old: AmeData<crate::v14::Actions>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Actions>> {
    Ok(AmeData::<Actions> {
        sidebar: migrate_field!(_ctx, old.sidebar),
    })
}
#[migrate(explicit)]
fn migrate_sidebaractions(
    old: AmeData<crate::v14::SidebarActions>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<SidebarActions>> {
    Ok(AmeData::<SidebarActions> {
        buffer: old.buffer,
        focused_buffer: old.focused_buffer,
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v14::Config>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        font: old.font,
        servers: old.servers,
        buffer: old.buffer,
        sidebar: migrate_field!(_ctx, old.sidebar),
        actions: migrate_field!(_ctx, old.actions),
        tooltips: old.tooltips,
    })
}
#[cfg(not(feature = "s16"))]
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
    assert_eq!(
        config.sidebar().order_by().get(),
        OrderBy::Config,
        "sidebar.order_by"
    );
    assert_eq!(
        config.sidebar().scrollbar().width().get(),
        9,
        "sidebar.scrollbar.width"
    );
    assert_eq!(
        config.sidebar().scrollbar().scroller_width().get(),
        3,
        "sidebar.scrollbar.scroller_width"
    );
    assert_eq!(
        config.actions().sidebar().buffer().get(),
        BufferAction::ReplacePane,
        "actions.sidebar.buffer"
    );
    assert_eq!(
        config.actions().sidebar().focused_buffer().get(),
        Some(FocusedAction::ClosePane),
        "actions.sidebar.focused_buffer"
    );
    assert!(!config.tooltips().get(), "tooltips");
}
#[cfg(not(feature = "s16"))]
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
        assert_eq!(
            config.sidebar().order_by().get(),
            OrderBy::Config,
            "sidebar.order_by"
        );
        assert_eq!(
            config.sidebar().scrollbar().width().get(),
            5,
            "sidebar.scrollbar.width"
        );
        assert_eq!(
            config.sidebar().scrollbar().scroller_width().get(),
            5,
            "sidebar.scrollbar.scroller_width"
        );
        assert_eq!(
            config.actions().sidebar().buffer().get(),
            BufferAction::ReplacePane,
            "actions.sidebar.buffer"
        );
        assert_eq!(
            config.actions().sidebar().focused_buffer().get(),
            Some(FocusedAction::ClosePane),
            "actions.sidebar.focused_buffer"
        );
        assert!(!config.tooltips().get(), "tooltips");
        config.sidebar().scrollbar().width().set(9);
        config.sidebar().scrollbar().scroller_width().set(3);
    }
    check(store);
}
