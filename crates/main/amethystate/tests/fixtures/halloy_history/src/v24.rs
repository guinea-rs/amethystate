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
    #[amestate(default = true)]
    pub show_on_open_buffers: bool,
    #[amestate(default = false)]
    pub query_as_highlight: bool,
}
#[amethystate]
pub struct HighlightIndicator {
    #[amestate(default = false)]
    pub title: bool,
    #[amestate(default = Icon::CircleEmpty)]
    pub icon: Icon,
    #[amestate(default = 6)]
    pub icon_size: u32,
    #[amestate(default = true)]
    pub show_on_open_buffers: bool,
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
    #[amestate(nested)]
    pub highlight_indicator: HighlightIndicator,
    #[amestate(default = Position::Left)]
    pub position: Position,
    #[amestate(default = OrderBy::Alpha)]
    pub order_by: OrderBy,
    #[amestate(nested)]
    pub scrollbar: Scrollbar,
    #[amestate(default = None)]
    pub secondary_font_size: Option<u8>,
    #[amestate(default = ServerIcon::Size(20))]
    pub primary_icon: ServerIcon,
    #[amestate(default = None)]
    pub primary_font_size: Option<u8>,
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
    #[amestate(default = CycleAction::IntoCollapsed)]
    pub cycle: CycleAction,
}
#[amethystate(prefix = "halloy", version = 24)]
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
    #[amestate(default = Tooltips::All)]
    pub tooltips: Tooltips,
    #[amestate(default = true)]
    pub check_for_update_on_launch: bool,
}
#[migrate(explicit)]
fn migrate_unreadindicator(
    old: AmeData<crate::v23::UnreadIndicator>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<UnreadIndicator>> {
    Ok(AmeData::<UnreadIndicator> {
        title: old.title,
        icon: old.icon,
        icon_size: old.icon_size,
        show_on_open_buffers: old.show_on_open_buffers,
        query_as_highlight: old.query_as_highlight,
    })
}
#[migrate(explicit)]
fn migrate_scrollbar(
    old: AmeData<crate::v23::Scrollbar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Scrollbar>> {
    Ok(AmeData::<Scrollbar> {
        width: old.width,
        scroller_width: old.scroller_width,
    })
}
#[migrate(explicit)]
fn migrate_usermenu(
    old: AmeData<crate::v23::UserMenu>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<UserMenu>> {
    Ok(AmeData::<UserMenu> {
        enabled: old.enabled,
    })
}
#[migrate(explicit)]
#[rename(font_size => secondary_font_size, server_icon => primary_icon, server_font_size => primary_font_size)]
fn migrate_sidebar(
    old: AmeData<crate::v23::Sidebar>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Sidebar>> {
    let highlight_indicator = AmeData::<HighlightIndicator> {
        title: old.unread_indicator.title,
        icon: old.unread_indicator.highlight_icon,
        icon_size: old.unread_indicator.highlight_icon_size,
        show_on_open_buffers: old.unread_indicator.show_on_open_buffers,
    };
    Ok(AmeData::<Sidebar> {
        max_width: old.max_width,
        unread_indicator: migrate_field!(_ctx, old.unread_indicator),
        highlight_indicator,
        position: old.position,
        order_by: old.order_by,
        scrollbar: migrate_field!(_ctx, old.scrollbar),
        secondary_font_size: old.font_size,
        primary_icon: old.server_icon,
        primary_font_size: old.server_font_size,
        user_menu: migrate_field!(_ctx, old.user_menu),
    })
}
#[migrate(explicit)]
fn migrate_actions(
    old: AmeData<crate::v23::Actions>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Actions>> {
    Ok(AmeData::<Actions> {
        sidebar: migrate_field!(_ctx, old.sidebar),
    })
}
#[migrate(explicit)]
fn migrate_sidebaractions(
    old: AmeData<crate::v23::SidebarActions>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<SidebarActions>> {
    Ok(AmeData::<SidebarActions> {
        buffer: old.buffer,
        channel: old.channel,
        query: old.query,
        focused_buffer: old.focused_buffer,
        cycle: CycleAction::IntoCollapsed,
    })
}
#[migrate]
fn migrate_config(
    old: AmeData<crate::v23::Config>,
    _ctx: &mut MigrationContext,
) -> MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        font: old.font,
        servers: old.servers,
        buffer: old.buffer,
        sidebar: migrate_field!(_ctx, old.sidebar),
        actions: migrate_field!(_ctx, old.actions),
        tooltips: old.tooltips,
        check_for_update_on_launch: old.check_for_update_on_launch,
    })
}
#[cfg(not(feature = "s25"))]
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
    assert!(
        !config
            .sidebar()
            .unread_indicator()
            .show_on_open_buffers()
            .get(),
        "sidebar.unread_indicator.show_on_open_buffers"
    );
    assert!(
        config
            .sidebar()
            .unread_indicator()
            .query_as_highlight()
            .get(),
        "sidebar.unread_indicator.query_as_highlight"
    );
    assert!(
        config.sidebar().highlight_indicator().title().get(),
        "sidebar.highlight_indicator.title"
    );
    assert_eq!(
        config.sidebar().highlight_indicator().icon().get(),
        Icon::Speaker,
        "sidebar.highlight_indicator.icon"
    );
    assert_eq!(
        config.sidebar().highlight_indicator().icon_size().get(),
        11,
        "sidebar.highlight_indicator.icon_size"
    );
    assert!(
        !config
            .sidebar()
            .highlight_indicator()
            .show_on_open_buffers()
            .get(),
        "sidebar.highlight_indicator.show_on_open_buffers"
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
        config.sidebar().secondary_font_size().get(),
        Some(11),
        "sidebar.secondary_font_size"
    );
    assert_eq!(
        config.sidebar().primary_icon().get(),
        ServerIcon::Size(17),
        "sidebar.primary_icon"
    );
    assert_eq!(
        config.sidebar().primary_font_size().get(),
        Some(17),
        "sidebar.primary_font_size"
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
    assert_eq!(
        config.actions().sidebar().cycle().get(),
        CycleAction::SkipCollapsed,
        "actions.sidebar.cycle"
    );
    assert_eq!(config.tooltips().get(), Tooltips::None, "tooltips");
    assert!(
        !config.check_for_update_on_launch().get(),
        "check_for_update_on_launch"
    );
}
#[cfg(not(feature = "s25"))]
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
        assert!(
            !config
                .sidebar()
                .unread_indicator()
                .show_on_open_buffers()
                .get(),
            "sidebar.unread_indicator.show_on_open_buffers"
        );
        assert!(
            config
                .sidebar()
                .unread_indicator()
                .query_as_highlight()
                .get(),
            "sidebar.unread_indicator.query_as_highlight"
        );
        assert!(
            config.sidebar().highlight_indicator().title().get(),
            "sidebar.highlight_indicator.title"
        );
        assert_eq!(
            config.sidebar().highlight_indicator().icon().get(),
            Icon::Speaker,
            "sidebar.highlight_indicator.icon"
        );
        assert_eq!(
            config.sidebar().highlight_indicator().icon_size().get(),
            11,
            "sidebar.highlight_indicator.icon_size"
        );
        assert!(
            !config
                .sidebar()
                .highlight_indicator()
                .show_on_open_buffers()
                .get(),
            "sidebar.highlight_indicator.show_on_open_buffers"
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
            config.sidebar().secondary_font_size().get(),
            Some(11),
            "sidebar.secondary_font_size"
        );
        assert_eq!(
            config.sidebar().primary_icon().get(),
            ServerIcon::Size(17),
            "sidebar.primary_icon"
        );
        assert_eq!(
            config.sidebar().primary_font_size().get(),
            Some(17),
            "sidebar.primary_font_size"
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
        assert_eq!(
            config.actions().sidebar().cycle().get(),
            CycleAction::IntoCollapsed,
            "actions.sidebar.cycle"
        );
        assert_eq!(config.tooltips().get(), Tooltips::None, "tooltips");
        assert!(
            !config.check_for_update_on_launch().get(),
            "check_for_update_on_launch"
        );
        config
            .actions()
            .sidebar()
            .cycle()
            .set(CycleAction::SkipCollapsed);
    }
    check(store);
}
