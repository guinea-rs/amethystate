pub use amethystate::Store;
use amethystate::amethystate;
use serde::{Deserialize, Serialize};
pub use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BufferAction {
    #[default]
    NewPane,
    ReplacePane,
    NewWindow,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FocusedAction {
    #[default]
    ClosePane,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnreadChoice {
    #[default]
    Dot,
    Title,
    None,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Icon {
    #[default]
    Dot,
    CircleEmpty,
    DotCircled,
    Certificate,
    Asterisk,
    Speaker,
    Lightbulb,
    Star,
    None,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderBy {
    #[default]
    Alpha,
    Config,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tooltips {
    #[default]
    All,
    AutocompleteOnly,
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServerIcon {
    Size(u32),
    Hidden,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CycleAction {
    #[default]
    IntoCollapsed,
    SkipCollapsed,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Brackets {
    pub left: String,
    pub right: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Timestamp {
    pub format: String,
    pub brackets: Brackets,
}

#[amethystate]
pub struct Font {
    #[amestate(default = None)]
    pub family: Option<String>,
    #[amestate(default = None)]
    pub size: Option<u8>,
}
#[amethystate]
pub struct Buffer {
    #[amestate(default = Some(Timestamp { format: "%T".into(), brackets: Brackets::default() }))]
    pub timestamp: Option<Timestamp>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Server {
    pub nickname: String,
    pub server: String,
    pub channels: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloodServer {
    pub nickname: String,
    pub server: String,
    pub channels: Vec<String>,
    pub anti_flood: Duration,
}
impl Default for FloodServer {
    fn default() -> Self {
        Self {
            nickname: String::new(),
            server: String::new(),
            channels: Vec::new(),
            anti_flood: Duration::from_millis(2000),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AntiFlood {
    pub rate: Duration,
    pub burst: usize,
}
impl Default for AntiFlood {
    fn default() -> Self {
        Self {
            rate: Duration::from_millis(2000),
            burst: 8,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BurstServer {
    pub nickname: String,
    pub server: String,
    pub channels: Vec<String>,
    pub anti_flood: AntiFlood,
}

pub fn check_stable(font: &Font, buffer: &Buffer) {
    assert_eq!(font.family().get(), Some("User Mono".to_string()));
    assert_eq!(font.size().get(), Some(19));
    assert_eq!(
        buffer.timestamp().get(),
        Some(Timestamp {
            format: "%H:%M:%S".into(),
            brackets: Brackets {
                left: "[".into(),
                right: "]".into()
            }
        })
    );
}
#[cfg(not(feature = "s02"))]
pub fn initialize(font: &Font, buffer: &Buffer) {
    assert_eq!(font.family().get(), None);
    assert_eq!(font.size().get(), None);
    font.family().set(Some("User Mono".to_string()));
    font.size().set(Some(19));
    buffer.timestamp().set(Some(Timestamp {
        format: "%H:%M:%S".into(),
        brackets: Brackets {
            left: "[".into(),
            right: "]".into(),
        },
    }));
}
