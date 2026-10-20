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
    #[amestate(default = false)]
    pub query_as_highlight: bool,
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
    pub channel: Option<BufferAction>,
    #[amestate(default = None)]
    pub query: Option<BufferAction>,
    #[amestate(default = None)]
    pub focused_buffer: Option<FocusedAction>,
}
#[amethystate(prefix = "halloy", version = 21)]
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
    #[amestate(default = true)]
    pub check_for_update_on_launch: bool,
}
#[migrate(explicit)]
fn migrate_unreadindicator(
    old: AmeData<crate::v20::UnreadIndicator>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<UnreadIndicator>> {
    Ok(AmeData::<UnreadIndicator> {
        title: old.title,
        icon: old.icon,
        icon_size: old.icon_size,
        highlight_icon: old.highlight_icon,
        highlight_icon_size: old.highlight_icon_size,
        query_as_highlight: old.query_as_highlight,
    })
}
#[migrate(explicit)]
fn migrate_scrollbar(
    old: AmeData<crate::v20::Scrollbar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Scrollbar>> {
    Ok(AmeData::<Scrollbar> {
        width: old.width,
        scroller_width: old.scroller_width,
    })
}
#[migrate(explicit)]
fn migrate_usermenu(
    old: AmeData<crate::v20::UserMenu>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<UserMenu>> {
    Ok(AmeData::<UserMenu> {
        enabled: old.enabled,
    })
}
#[migrate(explicit)]
fn migrate_sidebar(
    old: AmeData<crate::v20::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    Ok(AmeData::<Sidebar> {
        max_width: old.max_width,
        unread_indicator: migrate_field!(_ctx, old.unread_indicator),
        position: old.position,
        order_by: old.order_by,
        scrollbar: migrate_field!(_ctx, old.scrollbar),
        server_icon: old.server_icon,
        user_menu: migrate_field!(_ctx, old.user_menu),
    })
}
#[migrate(explicit)]
fn migrate_actions(
    old: AmeData<crate::v20::Actions>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Actions>> {
    Ok(AmeData::<Actions> {
        sidebar: migrate_field!(_ctx, old.sidebar),
    })
}
#[migrate(explicit)]
fn migrate_sidebaractions(
    old: AmeData<crate::v20::SidebarActions>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<SidebarActions>> {
    Ok(AmeData::<SidebarActions> {
        buffer: old.buffer,
        channel: None,
        query: None,
        focused_buffer: old.focused_buffer,
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v20::Config>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Config>> {
    let check_for_update_on_launch = old.sidebar.user_menu.show_new_version_indicator;
    Ok(AmeData::<Config> {
        font: old.font,
        servers: old.servers,
        buffer: old.buffer,
        sidebar: migrate_field!(_ctx, old.sidebar),
        actions: migrate_field!(_ctx, old.actions),
        tooltips: old.tooltips,
        check_for_update_on_launch,
    })
}
#[cfg(not(feature = "s22"))]
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
    assert!(
        config
            .sidebar()
            .unread_indicator()
            .query_as_highlight()
            .get(),
        "sidebar.unread_indicator.query_as_highlight"
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
    assert_eq!(
        config.actions().sidebar().buffer().get(),
        BufferAction::ReplacePane,
        "actions.sidebar.buffer"
    );
    assert_eq!(
        config.actions().sidebar().channel().get(),
        Some(BufferAction::NewWindow),
        "actions.sidebar.channel"
    );
    assert_eq!(
        config.actions().sidebar().query().get(),
        Some(BufferAction::ReplacePane),
        "actions.sidebar.query"
    );
    assert_eq!(
        config.actions().sidebar().focused_buffer().get(),
        Some(FocusedAction::ClosePane),
        "actions.sidebar.focused_buffer"
    );
    assert!(!config.tooltips().get(), "tooltips");
    assert!(
        !config.check_for_update_on_launch().get(),
        "check_for_update_on_launch"
    );
}
#[cfg(not(feature = "s22"))]
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
        assert!(
            config
                .sidebar()
                .unread_indicator()
                .query_as_highlight()
                .get(),
            "sidebar.unread_indicator.query_as_highlight"
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
        assert_eq!(
            config.actions().sidebar().buffer().get(),
            BufferAction::ReplacePane,
            "actions.sidebar.buffer"
        );
        assert_eq!(
            config.actions().sidebar().channel().get(),
            None,
            "actions.sidebar.channel"
        );
        assert_eq!(
            config.actions().sidebar().query().get(),
            None,
            "actions.sidebar.query"
        );
        assert_eq!(
            config.actions().sidebar().focused_buffer().get(),
            Some(FocusedAction::ClosePane),
            "actions.sidebar.focused_buffer"
        );
        assert!(!config.tooltips().get(), "tooltips");
        assert!(
            !config.check_for_update_on_launch().get(),
            "check_for_update_on_launch"
        );
        config
            .actions()
            .sidebar()
            .channel()
            .set(Some(BufferAction::NewWindow));
        config
            .actions()
            .sidebar()
            .query()
            .set(Some(BufferAction::ReplacePane));
    }
    check(store);
}
