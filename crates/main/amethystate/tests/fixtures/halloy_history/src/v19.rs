use crate::common::*;
use amethystate::{
    AmeData, MigrationContext, MigrationResult, ReactiveMap, amethystate, migrate, migrate_field,
};
#[amethystate]
pub struct UnreadIndicator {
    #[amestate(default = false)]
    pub title: bool,
    #[amestate(default = Icon::Dot)]
    pub icon: Icon,
    #[amestate(default = 6)]
    pub icon_size: u32,
    #[amestate(default = Icon::CircleEmpty)]
    pub highlight_icon: Icon,
    #[amestate(default = 8)]
    pub highlight_icon_size: u32,
}
#[amethystate]
pub struct Scrollbar {
    #[amestate(default = 5)]
    pub width: u32,
    #[amestate(default = 5)]
    pub scroller_width: u32,
}
#[amethystate]
pub struct UserMenu {
    #[amestate(default = true)]
    pub enabled: bool,
    #[amestate(default = true)]
    pub show_new_version_indicator: bool,
}
#[amethystate]
pub struct Sidebar {
    #[amestate(default = None)]
    pub max_width: Option<u16>,
    #[amestate(nested)]
    pub unread_indicator: UnreadIndicator,
    #[amestate(default = Position::Left)]
    pub position: Position,
    #[amestate(default = OrderBy::Alpha)]
    pub order_by: OrderBy,
    #[amestate(nested)]
    pub scrollbar: Scrollbar,
    #[amestate(default = ServerIcon::Size(12))]
    pub server_icon: ServerIcon,
    #[amestate(nested)]
    pub user_menu: UserMenu,
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
#[amethystate(prefix = "halloy", version = 19)]
pub struct Config {
    #[amestate(nested)]
    pub font: Font,
    #[amestate(default = {})]
    pub servers: ReactiveMap<String, FloodServer>,
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
fn migrate_unreadindicator(
    old: AmeData<crate::v18::UnreadIndicator>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<UnreadIndicator>> {
    Ok(AmeData::<UnreadIndicator> {
        title: old.title,
        icon: old.icon,
        icon_size: old.icon_size,
        highlight_icon: old.highlight_icon,
        highlight_icon_size: old.highlight_icon_size,
    })
}
#[migrate(explicit)]
fn migrate_scrollbar(
    old: AmeData<crate::v18::Scrollbar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Scrollbar>> {
    Ok(AmeData::<Scrollbar> {
        width: old.width,
        scroller_width: old.scroller_width,
    })
}
#[migrate(explicit)]
fn migrate_usermenu(
    old: AmeData<crate::v18::UserMenu>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<UserMenu>> {
    Ok(AmeData::<UserMenu> {
        enabled: old.enabled,
        show_new_version_indicator: old.show_new_version_indicator,
    })
}
#[migrate(explicit)]
#[rename(server_icon_size => server_icon)]
fn migrate_sidebar(
    old: AmeData<crate::v18::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    Ok(AmeData::<Sidebar> {
        max_width: old.max_width,
        unread_indicator: migrate_field!(_ctx, old.unread_indicator),
        position: old.position,
        order_by: old.order_by,
        scrollbar: migrate_field!(_ctx, old.scrollbar),
        server_icon: ServerIcon::Size(old.server_icon_size),
        user_menu: migrate_field!(_ctx, old.user_menu),
    })
}
#[migrate(explicit)]
fn migrate_actions(
    old: AmeData<crate::v18::Actions>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Actions>> {
    Ok(AmeData::<Actions> {
        sidebar: migrate_field!(_ctx, old.sidebar),
    })
}
#[migrate(explicit)]
fn migrate_sidebaractions(
    old: AmeData<crate::v18::SidebarActions>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<SidebarActions>> {
    Ok(AmeData::<SidebarActions> {
        buffer: old.buffer,
        focused_buffer: old.focused_buffer,
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v18::Config>,
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
#[cfg(not(feature = "s20"))]
pub fn check(store: &Store) {
    let config = Config::new_with(store);
    check_stable(&config.font(), &config.buffer());
    assert_eq!(config.servers().len(), 2);
    assert_eq!(
        config.servers().get("libera.net"),
        Some(FloodServer {
            nickname: "ignat".into(),
            server: "irc.libera.chat".into(),
            channels: vec!["#rust".into(), "#halloy".into()],
            anti_flood: Duration::new(1, 800000000)
        })
    );
    assert_eq!(
        config.servers().get("oftc"),
        Some(FloodServer {
            nickname: "second".into(),
            server: "irc.oftc.net".into(),
            channels: vec![],
            anti_flood: Duration::new(3, 500000000)
        })
    );
    assert_eq!(
        config.sidebar().max_width().get(),
        Some(173),
        "sidebar.max_width"
    );
    assert!(
        config.sidebar().unread_indicator().title().get(),
        "sidebar.unread_indicator.title"
    );
    assert_eq!(
        config.sidebar().unread_indicator().icon().get(),
        Icon::Star,
        "sidebar.unread_indicator.icon"
    );
    assert_eq!(
        config.sidebar().unread_indicator().icon_size().get(),
        9,
        "sidebar.unread_indicator.icon_size"
    );
    assert_eq!(
        config.sidebar().unread_indicator().highlight_icon().get(),
        Icon::Speaker,
        "sidebar.unread_indicator.highlight_icon"
    );
    assert_eq!(
        config
            .sidebar()
            .unread_indicator()
            .highlight_icon_size()
            .get(),
        11,
        "sidebar.unread_indicator.highlight_icon_size"
    );
    assert_eq!(
        config.sidebar().position().get(),
        Position::Right,
        "sidebar.position"
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
        config.sidebar().server_icon().get(),
        ServerIcon::Size(17),
        "sidebar.server_icon"
    );
    assert!(
        !config.sidebar().user_menu().enabled().get(),
        "sidebar.user_menu.enabled"
    );
    assert!(
        !config
            .sidebar()
            .user_menu()
            .show_new_version_indicator()
            .get(),
        "sidebar.user_menu.show_new_version_indicator"
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
#[cfg(not(feature = "s20"))]
pub fn exercise(store: &Store, new_schema: bool) {
    if new_schema {
        let config = Config::new_with(store);
        assert_eq!(config.servers().len(), 2);
        assert_eq!(
            config.servers().get("libera.net"),
            Some(FloodServer {
                nickname: "ignat".into(),
                server: "irc.libera.chat".into(),
                channels: vec!["#rust".into(), "#halloy".into()],
                anti_flood: Duration::new(1, 800000000)
            })
        );
        assert_eq!(
            config.servers().get("oftc"),
            Some(FloodServer {
                nickname: "second".into(),
                server: "irc.oftc.net".into(),
                channels: vec![],
                anti_flood: Duration::new(3, 500000000)
            })
        );
        assert_eq!(
            config.sidebar().max_width().get(),
            Some(173),
            "sidebar.max_width"
        );
        assert!(
            config.sidebar().unread_indicator().title().get(),
            "sidebar.unread_indicator.title"
        );
        assert_eq!(
            config.sidebar().unread_indicator().icon().get(),
            Icon::Star,
            "sidebar.unread_indicator.icon"
        );
        assert_eq!(
            config.sidebar().unread_indicator().icon_size().get(),
            9,
            "sidebar.unread_indicator.icon_size"
        );
        assert_eq!(
            config.sidebar().unread_indicator().highlight_icon().get(),
            Icon::Speaker,
            "sidebar.unread_indicator.highlight_icon"
        );
        assert_eq!(
            config
                .sidebar()
                .unread_indicator()
                .highlight_icon_size()
                .get(),
            11,
            "sidebar.unread_indicator.highlight_icon_size"
        );
        assert_eq!(
            config.sidebar().position().get(),
            Position::Right,
            "sidebar.position"
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
            config.sidebar().server_icon().get(),
            ServerIcon::Size(17),
            "sidebar.server_icon"
        );
        assert!(
            !config.sidebar().user_menu().enabled().get(),
            "sidebar.user_menu.enabled"
        );
        assert!(
            !config
                .sidebar()
                .user_menu()
                .show_new_version_indicator()
                .get(),
            "sidebar.user_menu.show_new_version_indicator"
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
    check(store);
}
