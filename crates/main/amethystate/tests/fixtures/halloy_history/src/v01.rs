use crate::common::*;
use amethystate::{ReactiveMap, amethystate};
#[amethystate(prefix = "halloy", version = 1)]
pub struct Config {
    #[amestate(nested)]
    pub font: Font,
    #[amestate(default = {})]
    pub servers: ReactiveMap<String, Server>,
    #[amestate(nested)]
    pub new_buffer: Buffer,
}
#[cfg(not(feature = "s02"))]
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
}
#[cfg(not(feature = "s02"))]
pub fn exercise(store: &Store, new_schema: bool) {
    if new_schema {
        let config = Config::new_with(store);
        assert_eq!(config.servers().len(), 0);
        initialize(&config.font(), &config.new_buffer());
        config
            .servers()
            .insert(
                "libera.net".to_string(),
                &Server {
                    nickname: "ignat".into(),
                    server: "irc.libera.chat".into(),
                    channels: vec!["#rust".into(), "#halloy".into()],
                },
            )
            .unwrap();
        config
            .servers()
            .insert(
                "oftc".to_string(),
                &Server {
                    nickname: "second".into(),
                    server: "irc.oftc.net".into(),
                    channels: vec![],
                },
            )
            .unwrap();
    }
    check(store);
}
