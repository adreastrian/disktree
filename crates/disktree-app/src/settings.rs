//! What the person chose, kept between launches.
//!
//! One small `key = value` file at
//! `~/Library/Application Support/Disktree/settings`, read once before the
//! first palette is resolved and written on every change. The format is
//! deliberately plain: a key the app does not know, or a value it cannot
//! read, is ignored and the default stands, so a file written by a newer
//! Disktree never stops an older one from starting.
//!
//! The path is a global the app installs at launch. The tests install
//! `None`, which keeps everything in memory, so a test can never touch the
//! person's real file.

use std::io;
use std::path::{Path, PathBuf};

use gpui_kit::{App, Global};

use crate::theme::{self, AppearanceChoice, Flavour, ThemeChoice, ThemeId};

/// Where the choice is kept, or `None` to keep nothing.
#[derive(Debug)]
pub struct SettingsFile(pub Option<PathBuf>);
impl Global for SettingsFile {}

/// The file the app reads and writes: inside the person's own
/// `Application Support`, where macOS keeps an app's preferences that are
/// not a property list. `None` when `HOME` is unset, and then nothing is
/// saved.
pub fn default_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support/Disktree")
            .join("settings"),
    )
}

/// Read the choice from `path`, install it as the current one, and remember
/// the path for the saves to come. Called before the first `theme::refresh`
/// so the very first frame is already in the chosen palette.
pub fn init(path: Option<PathBuf>, cx: &mut App) {
    let choice = path.as_deref().map(load).unwrap_or_default();
    cx.set_global(SettingsFile(path));
    cx.set_global(choice);
}

/// Make `choice` the current one everywhere: the palette, the check marks
/// in the View menu, and the file for next time.
pub fn choose(choice: ThemeChoice, cx: &mut App) {
    theme::select(choice, cx);
    crate::menu::sync(cx);
    let path = cx
        .try_global::<SettingsFile>()
        .and_then(|file| file.0.clone());
    if let Some(path) = path
        && let Err(error) = save(&path, choice)
    {
        // A read-only home is not worth a dialog: the choice still applies
        // for this run, it is only not remembered.
        eprintln!(
            "disktree: cannot save settings to {}: {error}",
            path.display()
        );
    }
}

/// The choice in the file at `path`; the default when the file is missing
/// or cannot be read.
pub fn load(path: &Path) -> ThemeChoice {
    std::fs::read_to_string(path)
        .map(|text| parse(&text))
        .unwrap_or_default()
}

/// Write `choice` to `path`, creating the directory. The file is written
/// beside its final name and renamed into place, so a crash mid-write
/// leaves the old file, never a torn one.
pub fn save(path: &Path, choice: ThemeChoice) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "settings path has no parent",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings");
    // Same directory as the target: `rename` is atomic only on one volume.
    let temp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&temp, render(choice))?;
    std::fs::rename(&temp, path).inspect_err(|_| {
        // Best effort: the rename failing is the error worth reporting.
        let _ = std::fs::remove_file(&temp);
    })
}

/// The text of the settings file for `choice`.
pub fn render(choice: ThemeChoice) -> String {
    format!(
        "theme = {}\nflavour = {}\nappearance = {}\n",
        choice.theme.key(),
        choice.flavour.key(),
        choice.appearance.key()
    )
}

/// Read a settings file. Blank lines and `#` comments are skipped; a line
/// without `=`, an unknown key or an unknown value is ignored and the
/// default for that key stands.
pub fn parse(text: &str) -> ThemeChoice {
    let mut choice = ThemeChoice::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "theme" => {
                if let Some(theme) = ThemeId::from_key(value) {
                    choice.theme = theme;
                }
            }
            "flavour" => {
                if let Some(flavour) = Flavour::from_key(value) {
                    choice.flavour = flavour;
                }
            }
            "appearance" => {
                if let Some(appearance) = AppearanceChoice::from_key(value) {
                    choice.appearance = appearance;
                }
            }
            _ => {}
        }
    }
    choice
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_survives_the_round_trip() {
        let choice = ThemeChoice {
            theme: ThemeId::RosePine,
            flavour: Flavour::Mocha,
            appearance: AppearanceChoice::Dark,
        };
        assert_eq!(parse(&render(choice)), choice);
        for theme in ThemeId::ALL {
            for flavour in Flavour::ALL {
                for appearance in AppearanceChoice::ALL {
                    let choice = ThemeChoice {
                        theme,
                        flavour,
                        appearance,
                    };
                    assert_eq!(parse(&render(choice)), choice);
                }
            }
        }
    }

    #[test]
    fn unknown_lines_fall_back_to_the_defaults() {
        let text = "\
# a comment
theme = graphite
flavour = ristretto
appearance=dark
future-key = 42
not a setting
= no key
theme
";
        assert_eq!(
            parse(text),
            ThemeChoice {
                theme: ThemeId::Graphite,
                flavour: Flavour::default(),
                appearance: AppearanceChoice::Dark,
            }
        );
        assert_eq!(parse(""), ThemeChoice::default());
        assert_eq!(parse("\u{0}\u{ff}garbage"), ThemeChoice::default());
    }

    #[test]
    fn a_missing_file_is_the_default_and_saving_creates_the_directory() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let path = temp.path().join("Disktree").join("settings");
        assert_eq!(load(&path), ThemeChoice::default());

        let choice = ThemeChoice {
            theme: ThemeId::Aqua,
            flavour: Flavour::Macchiato,
            appearance: AppearanceChoice::Light,
        };
        save(&path, choice).expect("save");
        assert_eq!(load(&path), choice);
        // Nothing but the file is left behind: the temporary was renamed.
        let entries: Vec<_> = std::fs::read_dir(path.parent().expect("dir"))
            .expect("read dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("settings")]);

        // A second save replaces the first.
        save(&path, ThemeChoice::default()).expect("save again");
        assert_eq!(load(&path), ThemeChoice::default());
    }
}
