//! What a directory *is*, and whether its space can be had back.
//!
//! Colour in the treemap means a kind of data, and a hatch means reclaimable
//! space, so the two questions a cleanup tool exists to answer — "what is it"
//! and "can I delete it" — can be read off a tile at a glance.
//!
//! Both come from names. A short lookup of well-known directory names covers
//! most of a home directory; anything unmatched takes its parent's kind, and a
//! top-level directory with an unknown name takes the kind of its largest
//! recognisable child (`~/world` is mostly `.git`, so it is git). Reclaimable
//! space is the same idea, plus a sibling check where a name alone is too
//! common to trust: `target` is only a build directory beside a `Cargo.toml`.
//!
//! On macOS the interesting names live under `~/Library`, whose children are
//! named for what they hold (`Caches`, `Logs`, `Developer`) rather than for
//! the tool that wrote them, so a few rules also look at the parent's name:
//! `Devices` is a simulator's only under `CoreSimulator`.

use crate::tree::Node;

/// A kind of data, for colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Category {
    /// Source code and checkouts.
    Code,
    /// Space agents write into: worktrees, sandboxes, experiments.
    AgentScratch,
    /// Compilers, package managers and their installs.
    Toolchain,
    /// Folders a sync client owns.
    Synced,
    /// Version-control object stores.
    Git,
    /// Pictures, music, video, games and models.
    Media,
    /// Documents, downloads and the desktop.
    Documents,
    /// Caches and other regenerable state.
    Cache,
    /// Nothing recognisable.
    #[default]
    Other,
}

impl Category {
    /// The categories the legend lists, in its order.
    pub const LEGEND: [Self; 8] = [
        Self::Code,
        Self::AgentScratch,
        Self::Toolchain,
        Self::Synced,
        Self::Git,
        Self::Media,
        Self::Documents,
        Self::Cache,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Code => "Code",
            Self::AgentScratch => "Agent scratch",
            Self::Toolchain => "Toolchains",
            Self::Synced => "Synced",
            Self::Git => "Git",
            Self::Media => "Media",
            Self::Documents => "Documents",
            Self::Cache => "Cache",
            Self::Other => "Other",
        }
    }
}

/// Why a directory's space can be had back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reclaim {
    /// A cache: whatever wrote it will write it again.
    Regenerable,
    /// A sync client's old versions of files.
    SyncHistory,
    /// A package manager's content store.
    PackageStore,
    /// Compiler or bundler output beside its sources.
    BuildOutput,
    /// Installed dependencies beside their manifest.
    Reinstallable,
    /// Container or sandbox image layers.
    SandboxLayers,
    /// Sandbox or VM snapshots.
    Snapshots,
    /// Already deleted, still on disk.
    Trash,
    /// Scratch space meant to be thrown away.
    Temporary,
}

impl Reclaim {
    /// The reason, as the "Worth a look" list says it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Regenerable => "regenerable",
            Self::SyncHistory => "sync history",
            Self::PackageStore => "package store",
            Self::BuildOutput => "build output",
            Self::Reinstallable => "reinstallable",
            Self::SandboxLayers => "sandbox layers",
            Self::Snapshots => "snapshots",
            Self::Trash => "trash",
            Self::Temporary => "temporary",
        }
    }
}

/// Directory extensions Finder shows as a single file. Deleting half of one
/// leaves the rest unusable, so removal treats a package as one unit, and
/// so does the treemap's labelling.
const PACKAGE_EXTENSIONS: [&str; 21] = [
    "app",
    "photoslibrary",
    "musiclibrary",
    "imovielibrary",
    "fcpbundle",
    "logicx",
    "band",
    "xcarchive",
    "sparsebundle",
    "bundle",
    "framework",
    "pkg",
    "xcodeproj",
    "xcworkspace",
    "playground",
    "tvlibrary",
    "aplibrary",
    "keynote",
    "pages",
    "numbers",
    "rtfd",
];

/// The extension of a package bundle name, lowercased, if it has one. A bare
/// `.app` is a hidden directory called `app`, not a package.
fn package_extension(name: &str) -> Option<String> {
    let (stem, extension) = name.rsplit_once('.')?;
    if stem.is_empty() || extension.is_empty() {
        return None;
    }
    let extension = extension.to_ascii_lowercase();
    PACKAGE_EXTENSIONS
        .contains(&extension.as_str())
        .then_some(extension)
}

/// Whether a name is a macOS package bundle: a directory the Finder, and so
/// the person, thinks of as one file.
pub fn is_package_name(name: &str) -> bool {
    package_extension(name).is_some()
}

/// The kind a package bundle's extension announces, if any.
///
/// Applications go under [`Category::Toolchain`] — "installed software" —
/// because there is no category for apps yet, and a `.app` is reinstalled
/// from the store or the vendor the same way a Homebrew formula is. Media
/// libraries are media; project bundles are code; iWork bundles are
/// documents. Everything else (`.bundle`, `.pkg`, `.sparsebundle`) is
/// judged by where it is, since the same extension holds a plug-in, an
/// installer or a Time Machine image.
pub fn package_category(name: &str) -> Option<Category> {
    let category = match package_extension(name)?.as_str() {
        "app" | "framework" => Category::Toolchain,
        "photoslibrary" | "musiclibrary" | "imovielibrary" | "fcpbundle"
        | "logicx" | "band" | "tvlibrary" | "aplibrary" => Category::Media,
        "xcarchive" | "xcodeproj" | "xcworkspace" | "playground" => {
            Category::Code
        }
        "keynote" | "pages" | "numbers" | "rtfd" => Category::Documents,
        _ => return None,
    };
    Some(category)
}

/// The kind a directory name announces on its own, if any.
pub fn category_of_name(name: &str) -> Option<Category> {
    category_within(name, "")
}

/// The kind a directory name announces, given the name of the directory
/// holding it. `parent` is only consulted for names too generic to trust
/// alone: `Archives` is Xcode's only under `Xcode`.
fn category_within(name: &str, parent: &str) -> Option<Category> {
    let lower = name.to_ascii_lowercase();
    let parent = parent.to_ascii_lowercase();
    // Each kind is one arm, its names grouped by the reason they belong.
    let category = match lower.as_str() {
        "src" | "code" | "projects" | "repos" | "dev" | "work"
        | "workspace" | "workspaces" | "github.com" | "gitlab.com"
        | "sites" | "development"
        // Xcode's build products are output beside sources in spirit,
        // like `target`, though they live under ~/Library/Developer.
        | "deriveddata" => Category::Code,
        // Xcode's archives hold the dSYMs that symbolicate shipped crash
        // reports: code, never disposable, and only Xcode's under `Xcode`.
        "archives" if parent == "xcode" => Category::Code,
        ".codex" | ".claude" | ".cursor" | ".aider" | ".gemini"
        | ".continue" | ".windsurf" | ".agents" | ".openai" | "worktrees"
        | "experiments" | "scratch" | "playground" => Category::AgentScratch,
        ".cargo" | ".rustup" | ".local" | ".npm" | ".pnpm-store" | "pnpm"
        | ".bun" | ".deno" | "go" | ".gradle" | ".m2" | ".platformio"
        | "mise" | ".mise" | ".pyenv" | ".nvm" | ".gem" | "gem" | ".rbenv"
        | ".espressif" | ".arduino15" | ".config" | ".vscode" | ".zig"
        | ".rye" | ".conda" | "anaconda3" | "miniconda3" | ".opam"
        | ".ghcup" | ".stack" | ".julia" | ".dotnet" | ".android"
        | ".sdkman" | ".volta" | ".yarn" | ".java"
        // macOS: Homebrew (`/opt/homebrew`, `/usr/local/Homebrew`, and
        // the `Cellar`/`Caskroom` it installs into), the SDKs and
        // simulators under ~/Library/Developer, Android Studio's SDK
        // under ~/Library, and container runtimes, whose disk images are
        // their installs.
        | "homebrew" | "cellar" | "caskroom" | "developer" | "android"
        | "coresimulator" | ".cocoapods" | ".swiftpm" | ".orbstack"
        | ".docker" | ".colima" | ".lima" | "com.docker.docker"
        // Device symbol caches Xcode copies from every connected device.
        | "ios devicesupport" | "watchos devicesupport"
        | "tvos devicesupport" | "visionos devicesupport"
        | "macos devicesupport"
        // Installed software has no category of its own; it is closest
        // to a toolchain's installs, and colouring /Applications like
        // Homebrew keeps "what is installed" one colour.
        | "applications" => Category::Toolchain,
        "sync" | "dropbox" | "nextcloud" | "google drive" | "onedrive"
        | "pclouddrive" | "mega" | ".stversions"
        // iCloud Drive lives in ~/Library/Mobile Documents; the File
        // Provider clients (Dropbox, OneDrive, Google Drive) mount under
        // ~/Library/CloudStorage.
        | "mobile documents" | "cloudstorage" | "icloud drive"
        | "icloud drive (archive)" => Category::Synced,
        ".git" => Category::Git,
        "pictures" | "photos" | "music" | "videos" | "movies" | "steam"
        | "models" | ".ollama" | ".lmstudio" | "games" => Category::Media,
        "documents" | "desktop" | "downloads" | "books" | "notes"
        | "obsidian" | "public" | "templates"
        // Finder's device backups: the only copy of a phone's data, so
        // documents, and never hatched.
        | "mobilesync" => Category::Documents,
        ".cache" | "cache" | "caches" | ".ccache" | ".sccache" | "_cacache"
        | "__pycache__" | "node_modules" | "trash" | ".trash" | ".trashes"
        | "tmp" | ".tmp" | ".temporaryitems" | "logs" => Category::Cache,
        // ~/Library and the per-app state under it are app data of no
        // single kind. Said explicitly so a top-level ~/Library is not
        // coloured after its largest child, which is usually `Caches`.
        "library" | "containers" | "group containers"
        | "application support" => Category::Other,
        // Containers are named by bundle identifier, and `com.vendor.app`
        // is an identifier, not an application package.
        _ if parent == "containers" || parent == "group containers" => {
            return None;
        }
        _ => return package_category(name),
    };
    Some(category)
}

/// Whether a directory's space can be had back, judged from its name, the
/// kind of the directory holding it, and its siblings' names.
pub fn reclaim_of(
    name: &str,
    parent: Category,
    has_sibling: impl Fn(&str) -> bool,
) -> Option<Reclaim> {
    reclaim_within(name, "", parent, has_sibling)
}

/// [`reclaim_of`], also given the name of the directory holding it, for the
/// names that are only disposable under one particular parent.
fn reclaim_within(
    name: &str,
    parent_name: &str,
    parent: Category,
    has_sibling: impl Fn(&str) -> bool,
) -> Option<Reclaim> {
    let lower = name.to_ascii_lowercase();
    let parent_name = parent_name.to_ascii_lowercase();
    let reclaim = match lower.as_str() {
        ".cache" | "cache" | "caches" | ".ccache" | ".sccache" | "_cacache"
        // CocoaPods' specs checkout comes back with `pod repo update`.
        | ".cocoapods" => Reclaim::Regenerable,
        ".stversions" => Reclaim::SyncHistory,
        ".pnpm-store" => Reclaim::PackageStore,
        // pnpm keeps its global installs beside the store under
        // ~/Library/pnpm, so only the store itself is a package store.
        "store" if parent_name == "pnpm" => Reclaim::PackageStore,
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache"
        | ".next" | ".turbo" | ".parcel-cache" | "deriveddata" => {
            Reclaim::BuildOutput
        }
        // Too common to trust alone: only a build directory beside a manifest.
        "target" if has_sibling("Cargo.toml") => Reclaim::BuildOutput,
        ".build" if has_sibling("Package.swift") => Reclaim::BuildOutput,
        "build" if parent_name == "carthage" => Reclaim::BuildOutput,
        "node_modules" if has_sibling("package.json") => Reclaim::Reinstallable,
        "pods" if has_sibling("Podfile") => Reclaim::Reinstallable,
        // Xcode fetches device symbols again the next time a device of
        // that version is plugged in; gigabytes per version.
        "ios devicesupport" | "watchos devicesupport" | "tvos devicesupport"
        | "visionos devicesupport" | "macos devicesupport" => {
            Reclaim::Reinstallable
        }
        // A simulator is recreated from its runtime; `Devices` is only
        // the simulators' under `CoreSimulator`.
        "devices" if parent_name == "coresimulator" => Reclaim::Reinstallable,
        // Layers and snapshots are only disposable inside sandbox state.
        "layers" if parent == Category::AgentScratch => Reclaim::SandboxLayers,
        "snapshots" if parent == Category::AgentScratch => Reclaim::Snapshots,
        // `.Trashes` is the per-user trash at the root of another volume.
        "trash" | ".trash" | ".trashes" => Reclaim::Trash,
        // Logs are diagnostics nothing reads back; `.TemporaryItems` is
        // where Finder and apps stage copies at a volume's root.
        "tmp" | ".tmp" | ".temporaryitems" | "logs" => Reclaim::Temporary,
        _ => return None,
    };
    Some(reclaim)
}

/// Assign a category and a reclaim reason to every node beneath `root`.
///
/// Top-down: a node's own name wins, otherwise it inherits. Reclaimable
/// space is inherited too, so everything under a cache is hatched.
pub fn classify(root: &mut Node) {
    root.category = Category::Other;
    root.reclaim = None;
    let root_name = root.name.clone();
    let children = std::mem::take(&mut root.children);
    let names: Vec<Box<str>> =
        children.iter().map(|child| child.name.clone()).collect();
    root.children = children;
    for index in 0..root.children.len() {
        let has_sibling =
            |wanted: &str| names.iter().any(|name| &**name == wanted);
        let child = &mut root.children[index];
        // A top-level directory with an unknown name takes the kind of its
        // largest recognisable child: `~/world` is mostly `.git`.
        let category = category_within(&child.name, &root_name)
            .or_else(|| is_git_store(child).then_some(Category::Git))
            .or_else(|| dominant_child_category(child))
            .unwrap_or(Category::Other);
        let reclaim = child
            .is_dir()
            .then(|| {
                reclaim_within(
                    &child.name,
                    &root_name,
                    Category::Other,
                    has_sibling,
                )
            })
            .flatten();
        classify_below(child, category, reclaim);
    }
}

fn classify_below(
    node: &mut Node,
    category: Category,
    reclaim: Option<Reclaim>,
) {
    node.category = category;
    node.reclaim = reclaim;
    if node.children.is_empty() {
        return;
    }
    let parent_name = node.name.clone();
    let names: Vec<Box<str>> = node
        .children
        .iter()
        .map(|child| child.name.clone())
        .collect();
    for child in &mut node.children {
        let has_sibling =
            |wanted: &str| names.iter().any(|name| &**name == wanted);
        let child_category = if child.is_dir() {
            category_within(&child.name, &parent_name)
                .or_else(|| is_git_store(child).then_some(Category::Git))
                .unwrap_or(category)
        } else {
            category
        };
        let child_reclaim = reclaim.or_else(|| {
            child
                .is_dir()
                .then(|| {
                    reclaim_within(
                        &child.name,
                        &parent_name,
                        category,
                        has_sibling,
                    )
                })
                .flatten()
        });
        classify_below(child, child_category, child_reclaim);
    }
}

/// The kind of an unknown directory, from what fills it: the first
/// recognisable name down the largest children, a few levels deep.
fn dominant_child_category(node: &Node) -> Option<Category> {
    let mut node = node;
    for _ in 0..3 {
        if let Some(category) = node
            .children
            .iter()
            .filter(|child| child.is_dir())
            .find_map(|child| {
                category_within(&child.name, &node.name)
                    .or_else(|| is_git_store(child).then_some(Category::Git))
            })
        {
            return Some(category);
        }
        node = node.children.iter().find(|child| child.is_dir())?;
    }
    None
}

/// A git object store by its shape, whatever it is called: a bare
/// repository, or a `.git` directory, has `objects`, `refs` and `HEAD`.
pub fn is_git_store(node: &Node) -> bool {
    let has =
        |wanted: &str| node.children.iter().any(|child| &*child.name == wanted);
    node.is_dir() && has("objects") && has("refs") && has("HEAD")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Metric, NodeKind, aggregate};

    fn file(name: &str, bytes: u64) -> Node {
        Node::entry(name, NodeKind::File, bytes)
    }

    fn dir(name: &str, children: Vec<Node>) -> Node {
        let mut node = Node::directory(name);
        node.children = children;
        node
    }

    fn scan(mut root: Node) -> Node {
        aggregate(&mut root, Metric::Bytes);
        classify(&mut root);
        root
    }

    fn home() -> Node {
        scan(dir(
            "tobi",
            vec![
                dir(
                    "src",
                    vec![dir(
                        "experiments",
                        vec![dir("2026-09-01", vec![file("a", 1)])],
                    )],
                ),
                dir(
                    ".cache",
                    vec![dir("kache", vec![dir("store", vec![file("b", 1)])])],
                ),
                dir(
                    "world",
                    vec![
                        dir(".git", vec![dir("objects", vec![file("c", 9)])]),
                        file("README", 1),
                    ],
                ),
                dir(
                    "rust-thing",
                    vec![
                        file("Cargo.toml", 1),
                        dir("target", vec![file("d", 5)]),
                    ],
                ),
                dir("js-thing", vec![dir("target", vec![file("e", 5)])]),
                dir(
                    ".codex",
                    vec![
                        dir("cache", vec![dir("layers", vec![file("f", 1)])]),
                        dir("snapshots", vec![file("g", 1)]),
                    ],
                ),
                dir("Sync", vec![dir(".stversions", vec![file("h", 1)])]),
                dir("mystery", vec![file("i", 1)]),
                dir(
                    "monorepo",
                    vec![dir(
                        "git",
                        vec![
                            file("HEAD", 1),
                            dir("refs", vec![]),
                            dir("objects", vec![file("pack", 50)]),
                        ],
                    )],
                ),
            ],
        ))
    }

    /// A macOS home directory, with the things that fill one.
    fn mac_home() -> Node {
        scan(dir(
            "arif",
            vec![
                dir(
                    "Library",
                    vec![
                        dir(
                            "Caches",
                            vec![dir("Homebrew", vec![file("bottle", 900)])],
                        ),
                        dir("Logs", vec![file("app.log", 10)]),
                        dir(
                            "Developer",
                            vec![
                                dir(
                                    "Xcode",
                                    vec![
                                        dir(
                                            "DerivedData",
                                            vec![dir(
                                                "App-abc",
                                                vec![file("o", 500)],
                                            )],
                                        ),
                                        dir(
                                            "Archives",
                                            vec![dir(
                                                "App.xcarchive",
                                                vec![file("dSYM", 100)],
                                            )],
                                        ),
                                        dir(
                                            "iOS DeviceSupport",
                                            vec![dir(
                                                "17.0",
                                                vec![file("Symbols", 400)],
                                            )],
                                        ),
                                    ],
                                ),
                                dir(
                                    "CoreSimulator",
                                    vec![
                                        dir(
                                            "Devices",
                                            vec![dir(
                                                "UUID",
                                                vec![file("data", 300)],
                                            )],
                                        ),
                                        dir(
                                            "Caches",
                                            vec![dir(
                                                "dyld",
                                                vec![file("cache", 200)],
                                            )],
                                        ),
                                    ],
                                ),
                            ],
                        ),
                        dir(
                            "Mobile Documents",
                            vec![dir(
                                "com~apple~CloudDocs",
                                vec![file("notes.md", 5)],
                            )],
                        ),
                        dir(
                            "CloudStorage",
                            vec![dir("Dropbox", vec![file("x", 5)])],
                        ),
                        dir(
                            "Containers",
                            vec![dir(
                                "com.example.app",
                                vec![dir(
                                    "Data",
                                    vec![dir(
                                        "Library",
                                        vec![
                                            dir("Caches", vec![file("c", 50)]),
                                            dir(
                                                "Application Support",
                                                vec![file("db", 60)],
                                            ),
                                        ],
                                    )],
                                )],
                            )],
                        ),
                        dir(
                            "Application Support",
                            vec![dir(
                                "MobileSync",
                                vec![dir(
                                    "Backup",
                                    vec![dir("phone", vec![file("b", 700)])],
                                )],
                            )],
                        ),
                        dir(
                            "pnpm",
                            vec![
                                dir("store", vec![file("v3", 80)]),
                                dir("global", vec![file("bin", 1)]),
                            ],
                        ),
                    ],
                ),
                dir(
                    "Pictures",
                    vec![dir(
                        "Photos Library.photoslibrary",
                        vec![dir("originals", vec![file("IMG_1", 999)])],
                    )],
                ),
                dir("Movies", vec![file("clip.mov", 40)]),
                dir(
                    "ios-app",
                    vec![
                        file("Podfile", 1),
                        dir("Pods", vec![file("lib", 30)]),
                    ],
                ),
                dir(
                    "swift-lib",
                    vec![
                        file("Package.swift", 1),
                        dir(".build", vec![file("o", 20)]),
                    ],
                ),
                dir("elsewhere", vec![dir(".build", vec![file("o", 20)])]),
                dir(".Trash", vec![file("old", 10)]),
            ],
        ))
    }

    /// `/opt` on a Mac with Homebrew.
    fn opt() -> Node {
        scan(dir(
            "opt",
            vec![dir(
                "homebrew",
                vec![
                    dir("Cellar", vec![dir("git", vec![file("bin", 10)])]),
                    dir("Caskroom", vec![file("x", 1)]),
                ],
            )],
        ))
    }

    fn named<'a>(node: &'a Node, path: &[&str]) -> &'a Node {
        let mut node = node;
        for part in path {
            node = node
                .children
                .iter()
                .find(|child| &*child.name == *part)
                .unwrap_or_else(|| panic!("no {part}"));
        }
        node
    }

    #[test]
    fn names_announce_their_kind_and_children_inherit_it() {
        let home = home();
        assert_eq!(named(&home, &["src"]).category, Category::Code);
        // `experiments` is agent scratch even inside code: it is what
        // agents write.
        assert_eq!(
            named(&home, &["src", "experiments"]).category,
            Category::AgentScratch
        );
        assert_eq!(
            named(&home, &["src", "experiments", "2026-09-01"]).category,
            Category::AgentScratch,
            "an unknown name inherits"
        );
        assert_eq!(
            named(&home, &["src", "experiments", "2026-09-01", "a"]).category,
            Category::AgentScratch,
            "files inherit too"
        );
    }

    #[test]
    fn an_unknown_top_level_directory_takes_its_largest_known_child() {
        let home = home();
        assert_eq!(named(&home, &["world"]).category, Category::Git);
        assert_eq!(named(&home, &["mystery"]).category, Category::Other);
        // A bare repository is git by its shape, whatever it is called.
        assert_eq!(named(&home, &["monorepo"]).category, Category::Git);
        assert_eq!(named(&home, &["monorepo", "git"]).category, Category::Git);
    }

    #[test]
    fn caches_are_reclaimable_all_the_way_down() {
        let home = home();
        assert_eq!(
            named(&home, &[".cache"]).reclaim,
            Some(Reclaim::Regenerable)
        );
        assert_eq!(
            named(&home, &[".cache", "kache", "store"]).reclaim,
            Some(Reclaim::Regenerable)
        );
        assert_eq!(
            named(&home, &["Sync", ".stversions"]).reclaim,
            Some(Reclaim::SyncHistory)
        );
        assert_eq!(named(&home, &["src"]).reclaim, None);
    }

    #[test]
    fn target_is_build_output_only_beside_a_cargo_manifest() {
        let home = home();
        assert_eq!(
            named(&home, &["rust-thing", "target"]).reclaim,
            Some(Reclaim::BuildOutput)
        );
        assert_eq!(named(&home, &["js-thing", "target"]).reclaim, None);
    }

    #[test]
    fn sandbox_layers_and_snapshots_are_reclaimable_only_in_sandbox_state() {
        let home = home();
        // .codex/cache is already a regenerable cache, so its layers
        // inherit that; the snapshots are judged on their own.
        assert!(
            named(&home, &[".codex", "cache", "layers"])
                .reclaim
                .is_some()
        );
        assert_eq!(
            named(&home, &[".codex", "snapshots"]).reclaim,
            Some(Reclaim::Snapshots)
        );
        assert_eq!(
            reclaim_of("snapshots", Category::Documents, |_| false),
            None
        );
    }

    #[test]
    fn library_is_app_data_and_its_caches_and_logs_are_disposable() {
        let home = mac_home();
        let library = named(&home, &["Library"]);
        // Not coloured after `Caches`, its largest child.
        assert_eq!(library.category, Category::Other);
        assert_eq!(library.reclaim, None);
        let caches = named(&home, &["Library", "Caches"]);
        assert_eq!(caches.category, Category::Cache);
        assert_eq!(caches.reclaim, Some(Reclaim::Regenerable));
        assert_eq!(
            named(&home, &["Library", "Caches", "Homebrew"]).reclaim,
            Some(Reclaim::Regenerable)
        );
        let logs = named(&home, &["Library", "Logs"]);
        assert_eq!(logs.category, Category::Cache);
        assert_eq!(logs.reclaim, Some(Reclaim::Temporary));
    }

    #[test]
    fn xcode_build_products_and_symbols_go_but_archives_stay() {
        let home = mac_home();
        let xcode = &["Library", "Developer", "Xcode"];
        assert_eq!(named(&home, xcode).category, Category::Toolchain);
        let derived =
            named(&home, &["Library", "Developer", "Xcode", "DerivedData"]);
        assert_eq!(derived.category, Category::Code);
        assert_eq!(derived.reclaim, Some(Reclaim::BuildOutput));
        let archives =
            named(&home, &["Library", "Developer", "Xcode", "Archives"]);
        assert_eq!(archives.category, Category::Code);
        assert_eq!(archives.reclaim, None, "dSYMs are not regenerable");
        assert_eq!(
            named(
                &home,
                &["Library", "Developer", "Xcode", "Archives", "App.xcarchive"]
            )
            .reclaim,
            None
        );
        let support = named(
            &home,
            &["Library", "Developer", "Xcode", "iOS DeviceSupport"],
        );
        assert_eq!(support.category, Category::Toolchain);
        assert_eq!(support.reclaim, Some(Reclaim::Reinstallable));
        // `Archives` is only Xcode's under Xcode.
        assert_eq!(category_of_name("Archives"), None);
    }

    #[test]
    fn simulator_devices_are_reinstallable_only_under_coresimulator() {
        let home = mac_home();
        let devices =
            named(&home, &["Library", "Developer", "CoreSimulator", "Devices"]);
        assert_eq!(devices.category, Category::Toolchain);
        assert_eq!(devices.reclaim, Some(Reclaim::Reinstallable));
        assert_eq!(
            named(&home, &["Library", "Developer", "CoreSimulator", "Caches"])
                .reclaim,
            Some(Reclaim::Regenerable)
        );
        assert_eq!(reclaim_of("Devices", Category::Toolchain, |_| false), None);
    }

    #[test]
    fn cloud_folders_are_synced_and_app_containers_are_not_hatched() {
        let home = mac_home();
        assert_eq!(
            named(&home, &["Library", "Mobile Documents"]).category,
            Category::Synced
        );
        assert_eq!(
            named(&home, &["Library", "CloudStorage", "Dropbox"]).category,
            Category::Synced
        );
        let container = &["Library", "Containers", "com.example.app", "Data"];
        let data = named(&home, container);
        assert_eq!(data.category, Category::Other);
        assert_eq!(data.reclaim, None);
        let inner_caches = named(
            &home,
            &[
                "Library",
                "Containers",
                "com.example.app",
                "Data",
                "Library",
                "Caches",
            ],
        );
        assert_eq!(inner_caches.reclaim, Some(Reclaim::Regenerable));
        let support = named(
            &home,
            &[
                "Library",
                "Containers",
                "com.example.app",
                "Data",
                "Library",
                "Application Support",
            ],
        );
        assert_eq!(support.reclaim, None);
    }

    #[test]
    fn device_backups_are_documents_and_never_reclaimable() {
        let home = mac_home();
        let backup = named(
            &home,
            &["Library", "Application Support", "MobileSync", "Backup"],
        );
        assert_eq!(backup.category, Category::Documents);
        assert_eq!(backup.reclaim, None);
    }

    #[test]
    fn only_the_pnpm_store_is_a_package_store() {
        let home = mac_home();
        assert_eq!(
            named(&home, &["Library", "pnpm"]).category,
            Category::Toolchain
        );
        assert_eq!(named(&home, &["Library", "pnpm"]).reclaim, None);
        assert_eq!(
            named(&home, &["Library", "pnpm", "store"]).reclaim,
            Some(Reclaim::PackageStore)
        );
        assert_eq!(named(&home, &["Library", "pnpm", "global"]).reclaim, None);
    }

    #[test]
    fn media_folders_and_the_photos_library_are_media() {
        let home = mac_home();
        assert_eq!(named(&home, &["Movies"]).category, Category::Media);
        let photos =
            named(&home, &["Pictures", "Photos Library.photoslibrary"]);
        assert_eq!(photos.category, Category::Media);
        assert_eq!(photos.reclaim, None);
        assert_eq!(
            named(
                &home,
                &["Pictures", "Photos Library.photoslibrary", "originals"]
            )
            .category,
            Category::Media
        );
    }

    #[test]
    fn dependency_and_build_directories_need_their_manifest_beside_them() {
        let home = mac_home();
        assert_eq!(
            named(&home, &["ios-app", "Pods"]).reclaim,
            Some(Reclaim::Reinstallable)
        );
        assert_eq!(
            named(&home, &["swift-lib", ".build"]).reclaim,
            Some(Reclaim::BuildOutput)
        );
        assert_eq!(named(&home, &["elsewhere", ".build"]).reclaim, None);
        assert_eq!(reclaim_of("Pods", Category::Code, |_| false), None);
    }

    #[test]
    fn trash_is_trash_on_every_volume() {
        let home = mac_home();
        assert_eq!(named(&home, &[".Trash"]).reclaim, Some(Reclaim::Trash));
        assert_eq!(
            reclaim_of(".Trashes", Category::Other, |_| false),
            Some(Reclaim::Trash)
        );
    }

    #[test]
    fn homebrew_is_a_toolchain_wherever_it_is_installed() {
        let opt = opt();
        assert_eq!(named(&opt, &["homebrew"]).category, Category::Toolchain);
        assert_eq!(
            named(&opt, &["homebrew", "Cellar", "git"]).category,
            Category::Toolchain
        );
        assert_eq!(named(&opt, &["homebrew", "Cellar"]).reclaim, None);
        assert_eq!(category_of_name("Applications"), Some(Category::Toolchain));
    }

    #[test]
    fn package_names_are_known_by_extension_whatever_the_case() {
        for name in [
            "Safari.app",
            "Photos Library.photoslibrary",
            "Music Library.musiclibrary",
            "Project.imovielibrary",
            "Cut.fcpbundle",
            "Song.logicx",
            "Jam.band",
            "App.xcarchive",
            "Backup.sparsebundle",
            "Plugin.bundle",
            "Foo.framework",
            "Installer.PKG",
            "App.xcodeproj",
            "App.xcworkspace",
            "Try.playground",
            "TV.tvlibrary",
            "Aperture.aplibrary",
            "Deck.Keynote",
            "Letter.pages",
            "Sheet.numbers",
            "Note.rtfd",
        ] {
            assert!(is_package_name(name), "{name}");
        }
        for name in [".app", "app", "src", "node_modules", "Photos Library"] {
            assert!(!is_package_name(name), "{name}");
        }
    }

    #[test]
    fn package_extensions_hint_at_a_kind() {
        assert_eq!(package_category("Safari.app"), Some(Category::Toolchain));
        assert_eq!(
            package_category("Photos Library.photoslibrary"),
            Some(Category::Media)
        );
        assert_eq!(package_category("App.xcodeproj"), Some(Category::Code));
        assert_eq!(package_category("Deck.keynote"), Some(Category::Documents));
        // The same extension holds an installer, a plug-in or a disk image:
        // judged by where it is.
        assert_eq!(package_category("Installer.pkg"), None);
        assert_eq!(package_category("Backup.sparsebundle"), None);
        assert_eq!(package_category("src"), None);
    }

    #[test]
    fn the_legend_lists_every_named_category_once() {
        let mut seen = std::collections::HashSet::new();
        for category in Category::LEGEND {
            assert!(seen.insert(category));
            assert_ne!(category, Category::Other);
            assert!(!category.label().is_empty());
        }
    }
}
