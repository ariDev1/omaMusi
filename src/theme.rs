//! Read the current Omarchy palette and terminal font without changing configuration.
use gtk::gdk::RGBA;
use std::{collections::BTreeMap, fs, path::Path};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub colors: BTreeMap<String, String>,
    pub font: String,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: [
                ("background", "#1a1b26"),
                ("foreground", "#c0caf5"),
                ("accent", "#7aa2f7"),
                ("dark_foreground", "#565f89"),
                ("light_foreground", "#a9b1d6"),
                ("cyan", "#7dcfff"),
                ("green", "#9ece6a"),
                ("bright_foreground", "#ffffff"),
                ("magenta", "#bb9af7"),
            ]
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect(),
            font: "monospace".into(),
        }
    }
}
impl Theme {
    pub fn load() -> Self {
        Self::load_from(&gtk::glib::home_dir())
    }
    pub fn load_from(home: &Path) -> Self {
        let mut theme = Self::default();
        for suffix in [
            ".local/state/omarchy/current/theme/colors.toml",
            ".config/omarchy/current/theme/colors.toml",
        ] {
            if let Ok(text) = fs::read_to_string(home.join(suffix))
                && let Ok(data) = text.parse::<toml::Table>()
            {
                for (name, value) in data {
                    if let Some(value) = value.as_str()
                        && RGBA::parse(value).is_ok()
                    {
                        theme.colors.insert(name, value.to_owned());
                    }
                }
                break;
            }
        }
        if let Ok(text) = fs::read_to_string(home.join(".config/alacritty/alacritty.toml"))
            && let Ok(data) = text.parse::<toml::Table>()
            && let Some(font) = data
                .get("font")
                .and_then(|v| v.get("normal"))
                .and_then(|v| v.get("family"))
                .and_then(|v| v.as_str())
            && !font.is_empty()
        {
            theme.font = font.to_owned();
        }
        theme
    }
    pub fn palette(&self) -> [[f32; 3]; 6] {
        std::array::from_fn(|i| {
            let name = [
                "background",
                "accent",
                "cyan",
                "bright_foreground",
                "green",
                "magenta",
            ][i];
            let color = RGBA::parse(&self.colors[name]).unwrap_or(RGBA::WHITE);
            [color.red(), color.green(), color.blue()]
        })
    }
    pub fn css(&self) -> String {
        let font = self.font.replace(['\'', '"', '\\', '\n', '\r'], "");
        let c = &self.colors;
        format!(
            r#"
            window.omamusi {{ background-color: transparent; color: {}; font-family: '{}'; font-size: 14px; }}
            window.omamusi-playlists {{ background-color: {}; color: {}; font-family: '{}'; font-size: 14px; }}
            .omamusi-stage, .omamusi-list, .omamusi-list row, .omamusi-list scrolledwindow, .omamusi-entry {{ background: transparent; border: none; box-shadow: none; }}
            .omamusi-list row {{ padding: 5px 0; min-height: 16px; color: {}; }}
            .omamusi-list row:selected, .omamusi-list row:hover {{ background: {}; color: #000000; }}
            .omamusi-entry {{ padding: 0; color: {}; }}
            .omamusi-accent {{ color: {}; }} .omamusi-muted {{ color: {}; }}
            .omamusi-hint {{ color: {}; font-size: 11px; }}
            .omamusi-title {{ font-size: 23px; color: {}; }}
            .omamusi-caption {{ font-size: 11px; }}
            .omamusi-header {{ background: {}; }}
        "#,
            c["foreground"],
            font,
            c["background"],
            c["foreground"],
            font,
            c["light_foreground"],
            c["accent"],
            c["foreground"],
            c["accent"],
            c["light_foreground"],
            c["dark_foreground"],
            c["foreground"],
            c["background"]
        )
    }
}
