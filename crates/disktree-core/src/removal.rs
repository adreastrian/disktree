//! Turning marked paths into deletions, safely.
//!
//! Three things matter here, in this order: never remove something the user did
//! not point at, never descend into a different volume, and always be able to
//! say what happened.
//!
//! Two mechanisms are offered:
//!
//! * [`RemovalMode::Permanent`] — `rm -rf` semantics, implemented with the
//!   standard library rather than by shelling out, so no path ever reaches a
//!   shell and no filename can be misread as an option.
//! * [`RemovalMode::Trash`] — move to the Finder's Trash through Foundation's
//!   `NSFileManager`, which picks the right Trash for the volume the file is
//!   on (`~/.Trash`, or `/Volumes/X/.Trashes/<uid>` for an external disk)
//!   without a permission prompt and without a shell.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::macos::fs::MetadataExt as _;
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use objc2_foundation::{NSFileManager, NSURL};

use crate::classify::is_package_name;

/// One path the user asked to remove.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub path: PathBuf,
    /// Bytes measured when the path was marked; used for the projection.
    pub bytes: u64,
    pub is_dir: bool,
    pub hidden: bool,
}

/// How a removal should be carried out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RemovalMode {
    /// Delete now. Not recoverable.
    #[default]
    Permanent,
    /// Move to the Trash so the removal can be undone.
    Trash,
}

impl RemovalMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Permanent => "Delete permanently",
            Self::Trash => "Move to Trash",
        }
    }

    pub const fn detail(self) -> &'static str {
        match self {
            Self::Permanent => "rm -rf: unrecoverable",
            Self::Trash => "recoverable until the Trash is emptied",
        }
    }
}

/// A target that will not be touched, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blocked {
    pub path: PathBuf,
    pub reason: String,
}

/// The work a [`RemovalMode`] will actually do.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    /// Targets to act on, with any target contained in another removed first.
    pub targets: Vec<Target>,
    /// Marked paths covered by a target above; reported, not acted on.
    pub covered: Vec<Target>,
    /// Marked paths that must not be touched.
    pub blocked: Vec<Blocked>,
}

impl Plan {
    pub fn bytes(&self) -> u64 {
        self.targets.iter().map(|target| target.bytes).sum()
    }

    pub const fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

/// Build the plan for `targets`, which must all live under `root`.
///
/// Paths outside `root` are blocked rather than removed: the tree the user was
/// looking at is the only thing they consented to act on.
pub fn plan(targets: &[Target], root: &Path) -> Plan {
    let root = normalize(root);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut plan = Plan::default();
    let mut accepted: Vec<Target> = Vec::new();

    for target in targets {
        let path = normalize(&target.path);
        if let Some(reason) = refuse(&path, &root, home.as_deref()) {
            plan.blocked.push(Blocked {
                path: target.path.clone(),
                reason,
            });
            continue;
        }
        accepted.push(Target {
            path,
            ..target.clone()
        });
    }

    // A path inside another target is removed with it. Keep the outer one and
    // report the inner one so the review screen can explain the nesting.
    accepted.sort_by(|left, right| left.path.cmp(&right.path));
    let mut outer: Vec<Target> = Vec::new();
    for target in accepted {
        if outer
            .iter()
            .any(|candidate| target.path.starts_with(&candidate.path))
        {
            plan.covered.push(target);
        } else {
            outer.push(target);
        }
    }
    plan.targets = outer;
    plan
}

/// Trees macOS owns, refused with everything inside them. A whole-disk scan
/// shows them because they are part of what fills the disk, but the system
/// volume is sealed, `/private/var/db` holds the databases the OS runs on,
/// and `/private/var/vm` is swap; removing any of it by hand breaks the
/// machine. Refused even where permissions would allow it.
///
/// `/usr` has one exception, `/usr/local/*`, which Homebrew on Intel owns;
/// `system_tree` carves it out.
const SYSTEM_TREES: [&str; 12] = [
    "/System",
    // Configuration files (hosts, sudoers, ssh) are root-owned but carry
    // no SIP flag; nothing in here is ever disk-hogging user data.
    "/private/etc",
    "/usr",
    "/bin",
    "/sbin",
    "/dev",
    "/private/var/db",
    "/private/var/vm",
    "/nix/store",
    "/Library/Apple",
    "/Library/Keychains",
    "/Library/Preferences/SystemConfiguration",
];

/// Directories refused as a whole but whose contents are the user's to
/// remove: `$TMPDIR` lives under `/private/var/folders`, Homebrew logs under
/// `/private/tmp`, third-party software under `/Library` and `/opt`.
const SYSTEM_DIRS: [&str; 9] = [
    "/private",
    "/private/var",
    "/private/tmp",
    "/private/var/folders",
    "/Library",
    "/Volumes",
    "/cores",
    "/opt",
    "/nix",
];

/// Directories under the home that macOS and its apps manage. The
/// directories themselves stay; what is inside them (a cache, a container's
/// data, a synced folder) is the user's to remove.
const HOME_DIRS: [&str; 8] = [
    "Library",
    "Library/Keychains",
    "Library/Preferences",
    "Library/Containers",
    "Library/Group Containers",
    "Library/Mobile Documents",
    "Library/CloudStorage",
    ".Trash",
];

/// The firmlink root: on macOS the boot volume group grafts the Data volume
/// onto the sealed system volume here, so `/Users/x` and
/// `/System/Volumes/Data/Users/x` are one directory.
const DATA_VOLUME: &str = "/System/Volumes/Data";

/// The classic Unix directories that are symlinks into `/private` on macOS.
/// They are fixed by the OS, so resolving them lexically is safe and keeps
/// `/etc/hosts` and `/private/etc/hosts` under the same guard.
const PRIVATE_ALIASES: [&str; 3] = ["/etc", "/var", "/tmp"];

/// Flag bits from `st_flags` (`sys/stat.h`) that `libc` does not export.
/// `SF_RESTRICTED` is System Integrity Protection: only entitled Apple
/// processes may write. `SF_NOUNLINK` marks the firmlink roots
/// (`/Applications`, `/Users`, `/Library`, …) that the kernel refuses to
/// unlink even for root.
const SF_RESTRICTED: u32 = 0x0008_0000;
const SF_NOUNLINK: u32 = 0x0010_0000;

/// One lexical form for paths that name the same directory: the Data volume
/// firmlink stripped, and `/etc`, `/var`, `/tmp` expanded into `/private`.
/// Every guard compares in this form so a path cannot dodge a rule by
/// arriving through the other name.
fn resolve_aliases(path: &Path) -> PathBuf {
    let stripped = match path.strip_prefix(DATA_VOLUME) {
        Ok(rest) if rest.as_os_str().is_empty() => PathBuf::from("/"),
        Ok(rest) => Path::new("/").join(rest),
        Err(_) => path.to_path_buf(),
    };
    for alias in PRIVATE_ALIASES {
        if let Ok(rest) = stripped.strip_prefix(alias) {
            return Path::new("/private").join(&alias[1..]).join(rest);
        }
    }
    stripped
}

/// The system tree `path` is in, if any. The home directory is never
/// system, wherever it lives, and `/usr/local`'s contents belong to Homebrew.
fn system_tree(path: &Path, home: Option<&Path>) -> Option<&'static str> {
    let path = resolve_aliases(path);
    if home.is_some_and(|home| path.starts_with(resolve_aliases(home))) {
        return None;
    }
    let local = Path::new("/usr/local");
    if path.starts_with(local) && path != local {
        return None;
    }
    SYSTEM_TREES
        .iter()
        .find(|tree| path.starts_with(tree))
        .copied()
}

/// Why this path must not be removed, if it must not.
fn refuse(path: &Path, root: &Path, home: Option<&Path>) -> Option<String> {
    if path.parent().is_none() {
        return Some("the filesystem root cannot be removed".into());
    }
    let resolved = resolve_aliases(path);
    if resolved.parent().is_none() {
        return Some("the filesystem root cannot be removed".into());
    }
    let root_resolved = resolve_aliases(root);
    if resolved == root_resolved {
        return Some("the scanned root cannot be removed".into());
    }
    let home = home.map(resolve_aliases);
    if home.as_deref() == Some(&*resolved) {
        return Some("the home directory cannot be removed".into());
    }
    if !resolved.starts_with(&root_resolved) {
        return Some("outside the scanned root".into());
    }

    // The kernel's own word comes first: the flags are set on exactly the
    // things Apple does not want touched, whatever they are called.
    if let Ok(meta) = fs::symlink_metadata(path) {
        let flags = meta.st_flags();
        if flags & SF_RESTRICTED != 0 {
            return Some(
                "protected by macOS (System Integrity Protection)".into(),
            );
        }
        if flags & SF_NOUNLINK != 0 {
            return Some(
                "macOS keeps this directory in place; it cannot be unlinked"
                    .into(),
            );
        }
    }
    if is_read_only_volume(path) {
        return Some("on a read-only volume".into());
    }

    if let Some(system) = system_tree(path, home.as_deref()) {
        return Some(format!(
            "part of macOS under {system}: macOS manages this; use Storage \
             settings or the app that owns it"
        ));
    }
    if SYSTEM_DIRS.iter().any(|dir| resolved == Path::new(dir)) {
        return Some(
            "a directory macOS lays out itself; remove what is inside it \
             instead"
                .into(),
        );
    }
    if is_volume_trash(&resolved) {
        return Some(
            "the Trash of a volume; empty it from the Finder instead".into(),
        );
    }
    if let Some(home) = &home {
        for dir in HOME_DIRS {
            if resolved == home.join(dir) {
                return Some(
                    "macOS manages this; use Storage settings or the app \
                     that owns it"
                        .into(),
                );
            }
        }
    }
    if is_mount_point(path) {
        return Some(
            "a mount point: removing it would cross onto another volume".into(),
        );
    }
    if let Some(reason) = inside_package(&resolved, &root_resolved) {
        return Some(reason);
    }
    None
}

/// `/Volumes/<disk>/.Trashes`, the per-volume Trash the Finder keeps.
fn is_volume_trash(resolved: &Path) -> bool {
    let mut parts = resolved.components();
    matches!(
        (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next()
        ),
        (
            Some(Component::RootDir),
            Some(Component::Normal(volumes)),
            Some(Component::Normal(_)),
            Some(Component::Normal(trashes)),
            None
        ) if volumes == "Volumes" && trashes == ".Trashes"
    )
}

/// Why a path inside a package bundle is refused, if it is one.
///
/// An `.app` or a library bundle is one thing to the Finder and to the app
/// that owns it; taking a file out of it leaves a broken package behind, so
/// only the package as a whole may be marked. The scanned root is exempt —
/// scanning *inside* a package is the user asking to look at its pieces —
/// except for a Photos library, whose database and originals must never be
/// split.
fn inside_package(resolved: &Path, root: &Path) -> Option<String> {
    let parent = resolved.parent()?;
    for ancestor in parent.ancestors() {
        let Some(name) = ancestor.file_name() else {
            continue;
        };
        let name = name.to_string_lossy();
        if !is_package_name(&name) {
            continue;
        }
        let below_root = ancestor != root && ancestor.starts_with(root);
        let photos = name.to_ascii_lowercase().ends_with(".photoslibrary");
        if below_root || photos {
            return Some(format!(
                "part of the {name} package; mark the package itself"
            ));
        }
    }
    None
}

/// The directory a volume is mounted on, as `statfs` reports it.
fn mount_name(stat: &rustix::fs::StatFs) -> PathBuf {
    let bytes: Vec<u8> = stat
        .f_mntonname
        .iter()
        .take_while(|&&byte| byte != 0)
        .map(|&byte| u8::from_ne_bytes(byte.to_ne_bytes()))
        .collect();
    PathBuf::from(OsStr::from_bytes(&bytes))
}

/// Whether the volume `path` lives on cannot be written to, like the sealed
/// system volume or a mounted disk image. Removing from it would fail
/// half-way; better to say so up front.
fn is_read_only_volume(path: &Path) -> bool {
    // A dangling symlink has no volume of its own; the unlink happens in the
    // parent, so the parent's volume is the one that matters.
    let stat = rustix::fs::statfs(path)
        .or_else(|error| path.parent().map_or(Err(error), rustix::fs::statfs));
    stat.is_ok_and(|stat| stat.f_flags & libc::MNT_RDONLY as u32 != 0)
}

/// Whether `path` is where a volume is mounted. Descending into one would
/// delete data the user never marked.
///
/// Device numbers cannot tell: the boot volume group shares one `st_dev`
/// across `/`, `/System` and the Data volume, so a directory is a mount
/// point only when `statfs` names it as the mount directory. Firmlinks such
/// as `/Users` report the Data volume's mount directory and are therefore
/// not mount points, which is right: the kernel refuses to unlink them
/// anyway.
pub fn is_mount_point(path: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.is_dir() {
        return false;
    }
    // A directory that cannot be resolved or queried is not worth the risk.
    let Ok(canonical) = fs::canonicalize(path) else {
        return true;
    };
    match rustix::fs::statfs(&canonical) {
        Ok(stat) => mount_name(&stat) == canonical,
        Err(_) => true,
    }
}

/// Lexically normalize a path: resolve `.` and `..` without touching the
/// filesystem, so a symlink can never redirect a guard.
///
/// A `..` at the root has nowhere to go, so `/..` is `/` rather than the empty
/// path. A leading `..` on a relative path is preserved.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(
                    out.components().next_back(),
                    Some(Component::Normal(_))
                ) {
                    out.pop();
                } else if !out.has_root() && out.as_os_str().is_empty() {
                    out.push("..");
                }
            }
            Component::Prefix(_)
            | Component::RootDir
            | Component::Normal(_) => {
                out.push(component.as_os_str());
            }
        }
    }
    out
}

/// What, if anything, moves files to the Trash.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrashBackend {
    /// The Finder's Trash, through Foundation.
    Finder,
    /// No way to move files to a Trash on this machine.
    #[default]
    Unavailable,
}

impl TrashBackend {
    pub const fn is_available(self) -> bool {
        match self {
            Self::Finder => true,
            Self::Unavailable => false,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Finder => "the Finder's Trash",
            Self::Unavailable => "no Trash available",
        }
    }

    pub const fn detail(self) -> &'static str {
        match self {
            Self::Finder => {
                "the same Trash the Finder uses; an external disk gets its own"
            }
            Self::Unavailable => "keep deleting permanently",
        }
    }
}

/// Detect the trash backend for this machine. Foundation is always present
/// on macOS, so this is the Finder's Trash.
pub const fn detect_trash_backend() -> TrashBackend {
    TrashBackend::Finder
}

/// What a running removal reports.
#[derive(Clone, Debug)]
pub enum RemovalEvent {
    Start {
        total: usize,
    },
    Item {
        path: PathBuf,
        bytes: u64,
        outcome: Result<(), String>,
    },
    Done {
        removed: u64,
        bytes: u64,
        failed: usize,
    },
}

/// A removal running on its own thread.
#[derive(Debug)]
pub struct RemovalHandle {
    events: Receiver<RemovalEvent>,
    cancel: Arc<AtomicBool>,
}

impl RemovalHandle {
    /// Take the next event, if one has arrived.
    pub fn poll(&self) -> Option<RemovalEvent> {
        // Empty and Disconnected mean the same thing to the caller: there is
        // nothing more to do this tick.
        self.events.try_recv().ok()
    }

    /// Stop before the next target starts.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Start removing the plan's targets on a worker thread.
pub fn spawn(plan: Plan, mode: RemovalMode) -> RemovalHandle {
    let (sender, events) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = Arc::clone(&cancel);
    let backend = if mode == RemovalMode::Trash {
        detect_trash_backend()
    } else {
        TrashBackend::Unavailable
    };

    let worker = thread::Builder::new()
        .name("disktree-remove".into())
        .spawn({
            let sender = sender.clone();
            move || run(&plan, mode, backend, &worker_cancel, &sender)
        });
    if let Err(error) = worker {
        let _ = sender.send(RemovalEvent::Item {
            path: PathBuf::new(),
            bytes: 0,
            outcome: Err(format!(
                "could not start the removal worker: {error}"
            )),
        });
    }

    RemovalHandle { events, cancel }
}

fn run(
    plan: &Plan,
    mode: RemovalMode,
    backend: TrashBackend,
    cancel: &AtomicBool,
    sender: &mpsc::Sender<RemovalEvent>,
) {
    let _ = sender.send(RemovalEvent::Start {
        total: plan.targets.len(),
    });
    let mut removed = 0_u64;
    let mut bytes = 0_u64;
    let mut failed = 0_usize;

    for target in &plan.targets {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let outcome = match mode {
            RemovalMode::Permanent => remove_permanently(&target.path),
            RemovalMode::Trash => {
                move_to_trash(&target.path, backend).map(|_| ())
            }
        };
        if outcome.is_ok() {
            removed += 1;
            bytes += target.bytes;
        } else {
            failed += 1;
        }
        let _ = sender.send(RemovalEvent::Item {
            path: target.path.clone(),
            bytes: target.bytes,
            outcome: outcome.map_err(|error| error.to_string()),
        });
    }

    let _ = sender.send(RemovalEvent::Done {
        removed,
        bytes,
        failed,
    });
}

/// `rm -rf` semantics: a symlink is unlinked, never followed.
pub fn remove_permanently(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Move one path to the Trash. Returns where it went when Foundation says.
pub fn move_to_trash(
    path: &Path,
    backend: TrashBackend,
) -> io::Result<Option<PathBuf>> {
    match backend {
        TrashBackend::Finder => trash_via_finder(path),
        TrashBackend::Unavailable => Err(io::Error::other(
            "no Trash is available; use permanent deletion instead",
        )),
    }
}

/// `NSCocoaErrorDomain`'s code for "this volume has no Trash", which some
/// network and read-only volumes report. Foundation's error list is behind
/// a feature of its own in `objc2-foundation`; the number is documented and
/// stable.
const NS_FEATURE_UNSUPPORTED_ERROR: isize = 3328;

/// Ask `NSFileManager` to trash `path`. It picks the Trash of the volume the
/// path is on, so an external disk's file lands in that disk's `.Trashes`
/// rather than being copied across.
fn trash_via_finder(path: &Path) -> io::Result<Option<PathBuf>> {
    // A missing path is reported as such rather than as Foundation's
    // wording, so the run log reads the same for both removal modes.
    fs::symlink_metadata(path)?;
    let url = NSURL::from_file_path(path).ok_or_else(|| {
        io::Error::other("the path cannot be expressed as a file URL")
    })?;
    let mut resulting = None;
    let manager = NSFileManager::defaultManager();
    match manager
        .trashItemAtURL_resultingItemURL_error(&url, Some(&mut resulting))
    {
        Ok(()) => Ok(resulting
            .and_then(|url| url.path())
            .map(|path| PathBuf::from(path.to_string()))),
        Err(error) => {
            let no_trash = error.code() == NS_FEATURE_UNSUPPORTED_ERROR
                && error.domain().to_string() == "NSCocoaErrorDomain";
            if no_trash {
                Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "this volume has no Trash; remove permanently instead",
                ))
            } else {
                Err(io::Error::other(error.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn target(path: &Path, bytes: u64) -> Target {
        Target {
            path: path.to_path_buf(),
            bytes,
            is_dir: path.is_dir(),
            hidden: false,
        }
    }

    fn tree() -> TempDir {
        let temp = TempDir::new().expect("tempdir");
        fs::create_dir_all(temp.path().join("a/b")).expect("mkdir");
        fs::write(temp.path().join("a/one.bin"), vec![b'x'; 10])
            .expect("write");
        fs::write(temp.path().join("a/c.bin"), vec![b'x'; 20]).expect("write");
        fs::create_dir_all(temp.path().join("other")).expect("mkdir");
        temp
    }

    fn home() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }

    #[test]
    fn a_plan_keeps_only_the_outermost_targets() {
        let temp = tree();
        let root = temp.path();
        let inner = target(&root.join("a/b"), 5);
        let outer = target(&root.join("a"), 30);
        let separate = target(&root.join("other"), 0);

        let plan =
            plan(&[inner.clone(), outer.clone(), separate.clone()], root);
        assert_eq!(plan.targets.len(), 2);
        assert!(plan.targets.contains(&outer));
        assert!(plan.targets.contains(&separate));
        assert_eq!(plan.covered.len(), 1);
        assert_eq!(plan.covered[0].path, inner.path);
        assert_eq!(plan.bytes(), 30);
    }

    #[test]
    fn the_root_and_home_are_refused() {
        let temp = tree();
        let root = temp.path();
        let home = home();
        let mut targets = vec![target(Path::new("/"), 0), target(root, 0)];
        if let Some(home) = &home {
            targets.push(target(home, 0));
        }

        let plan = plan(&targets, root);
        assert!(plan.is_empty());
        assert_eq!(plan.blocked.len(), targets.len());
        assert!(
            plan.blocked
                .iter()
                .any(|blocked| blocked.reason.contains("home directory"))
                || home.is_none()
        );
    }

    #[test]
    fn paths_outside_the_root_are_refused() {
        let temp = tree();
        let plan = plan(&[target(Path::new("/etc/passwd"), 1)], temp.path());
        assert!(plan.is_empty());
        assert_eq!(plan.blocked[0].reason, "outside the scanned root");
    }

    #[test]
    fn sibling_prefixes_are_not_treated_as_containment() {
        let temp = tree();
        let root = temp.path();
        fs::create_dir_all(root.join("a-real")).expect("mkdir");
        let plan = plan(
            &[target(&root.join("a"), 1), target(&root.join("a-real"), 1)],
            root,
        );
        assert_eq!(plan.targets.len(), 2, "a-real is not inside a");
        assert!(plan.covered.is_empty());
    }

    #[test]
    fn normalize_resolves_dots_without_the_filesystem() {
        assert_eq!(
            normalize(Path::new("/Users/tobi/./src/../src/")),
            PathBuf::from("/Users/tobi/src")
        );
        assert_eq!(normalize(Path::new("a/../b")), PathBuf::from("b"));
        assert_eq!(normalize(Path::new("/..")), PathBuf::from("/"));
        assert_eq!(normalize(Path::new("../x")), PathBuf::from("../x"));
    }

    #[test]
    fn mount_points_are_recognized() {
        let temp = tree();
        assert!(!is_mount_point(temp.path()));
        assert!(is_mount_point(Path::new("/")));
        // devfs is mounted on /dev on every Mac.
        assert!(is_mount_point(Path::new("/dev")));
        // The Data volume is a mount; /Users is a firmlink into it, and
        // /Volumes is a plain directory on the system volume.
        if Path::new(DATA_VOLUME).is_dir() {
            assert!(is_mount_point(Path::new(DATA_VOLUME)));
            assert!(!is_mount_point(Path::new("/Users")));
        }
        assert!(!is_mount_point(Path::new("/Volumes")));
        assert!(!is_mount_point(Path::new("/dev/null")), "not a directory");
    }

    #[test]
    fn permanent_removal_takes_directories_and_leaves_siblings() {
        let temp = tree();
        let doomed = temp.path().join("a/b");
        remove_permanently(&doomed).expect("remove dir");
        assert!(!doomed.exists());
        assert!(temp.path().join("a/c.bin").exists());
    }

    #[test]
    fn permanent_removal_unlinks_a_symlink_instead_of_following_it() {
        let temp = tree();
        let keep = TempDir::new().expect("tempdir");
        fs::write(keep.path().join("precious.bin"), b"data").expect("write");
        let link = temp.path().join("link");
        std::os::unix::fs::symlink(keep.path(), &link).expect("symlink");

        remove_permanently(&link).expect("remove link");
        assert!(!link.exists());
        assert!(keep.path().join("precious.bin").exists());
    }

    #[test]
    fn a_missing_target_reports_not_found() {
        let temp = tree();
        let error = remove_permanently(&temp.path().join("absent"))
            .expect_err("missing");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn a_removal_run_reports_every_item_and_a_total() {
        let temp = tree();
        let root = temp.path();
        let plan = plan(
            &[
                target(&root.join("a/b"), 0),
                target(&root.join("a/c.bin"), 20),
                target(&root.join("absent"), 0),
            ],
            root,
        );
        assert_eq!(plan.targets.len(), 3);

        let handle = spawn(plan, RemovalMode::Permanent);
        let mut events = Vec::new();
        for _ in 0..2000 {
            while let Some(event) = handle.poll() {
                events.push(event);
            }
            if matches!(events.last(), Some(RemovalEvent::Done { .. })) {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }

        assert!(matches!(events[0], RemovalEvent::Start { total: 3 }));
        let items = events
            .iter()
            .filter(|event| matches!(event, RemovalEvent::Item { .. }))
            .count();
        assert_eq!(items, 3);
        match events.last() {
            Some(RemovalEvent::Done {
                removed,
                bytes,
                failed,
            }) => {
                assert_eq!(*removed, 2);
                assert_eq!(*failed, 1);
                assert_eq!(*bytes, 20);
            }
            other => panic!("expected Done, got {other:?}"),
        }
        assert!(!root.join("a/b").exists());
        assert!(root.join("a/one.bin").exists());
    }

    #[test]
    fn the_finder_trash_takes_a_file_from_where_it_was() {
        // The Trash is the real ~/.Trash, so the name is unique to this run
        // and nothing of the user's is near it.
        let name = format!(
            "disktree-trash-test-{}-{}.bin",
            std::process::id(),
            chrono::Local::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
        );
        let file = std::env::temp_dir().join(&name);
        fs::write(&file, b"doomed").expect("write");

        let backend = detect_trash_backend();
        assert!(backend.is_available());
        let landed = move_to_trash(&file, backend).expect("trashed");

        assert!(!file.exists(), "gone from its origin");
        if let Some(landed) = landed {
            assert!(landed.to_string_lossy().contains("disktree-trash-test"));
            // ~/.Trash cannot be listed without Full Disk Access, but the
            // file is ours; unlinking it by name may still be refused, and
            // the Trash is the right place for leftovers anyway.
            let _ = fs::remove_file(landed);
        }
    }

    #[test]
    fn trashing_a_missing_path_reports_not_found() {
        let temp = TempDir::new().expect("tempdir");
        let error =
            move_to_trash(&temp.path().join("absent"), TrashBackend::Finder)
                .expect_err("missing");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn system_trees_are_refused_in_a_whole_disk_scan() {
        let home = Path::new("/Users/tobi");
        assert_eq!(
            system_tree(Path::new("/usr/lib/libfoo.dylib"), Some(home)),
            Some("/usr")
        );
        assert_eq!(
            system_tree(Path::new("/System/Library/Frameworks"), Some(home)),
            Some("/System")
        );
        assert_eq!(
            system_tree(Path::new("/private/var/db/dyld"), Some(home)),
            Some("/private/var/db")
        );
        assert_eq!(
            system_tree(Path::new("/var/db/dyld"), Some(home)),
            Some("/private/var/db"),
            "/var is /private/var"
        );
        assert_eq!(
            system_tree(Path::new("/private/var/folders/xy/z"), Some(home)),
            None
        );
        assert_eq!(system_tree(Path::new("/opt/thing"), Some(home)), None);
        assert_eq!(
            system_tree(Path::new("/usrlocal"), Some(home)),
            None,
            "components, not prefixes"
        );
        let odd_home = Path::new("/usr/home/tobi");
        assert_eq!(
            system_tree(Path::new("/usr/home/tobi/.cache"), Some(odd_home)),
            None
        );
        // The directories macOS lays out are refused as a whole, under
        // either name; what is inside them is the user's business.
        for dir in ["/private/etc", "/etc", "/private/var/folders"] {
            let reason = refuse(Path::new(dir), Path::new("/"), Some(home));
            assert!(
                reason.is_some_and(|reason| reason.contains("macOS")),
                "{dir}"
            );
        }
        // Configuration is a whole tree: nothing under /etc is user data.
        assert!(
            refuse(Path::new("/etc/hosts"), Path::new("/"), Some(home))
                .is_some()
        );
        assert_eq!(
            refuse(
                Path::new("/private/tmp/build.log"),
                Path::new("/"),
                Some(home)
            ),
            None
        );
        let reason = refuse(Path::new("/Volumes"), Path::new("/"), Some(home));
        assert!(reason.is_some_and(|reason| reason.contains("macOS")));
    }

    #[test]
    fn homebrew_under_usr_local_is_allowed_but_usr_bin_is_not() {
        let home = Path::new("/Users/tobi");
        assert_eq!(
            system_tree(Path::new("/usr/local/Cellar/foo"), Some(home)),
            None
        );
        assert_eq!(
            system_tree(Path::new("/usr/local"), Some(home)),
            Some("/usr")
        );
        assert_eq!(
            system_tree(Path::new("/usr/bin/true"), Some(home)),
            Some("/usr")
        );
        let reason =
            refuse(Path::new("/usr/bin/true"), Path::new("/"), Some(home));
        assert!(reason.is_some_and(|reason| reason.contains("macOS")));
    }

    #[test]
    fn firmlinked_paths_are_one_path_to_every_guard() {
        let data = Path::new(DATA_VOLUME);
        assert_eq!(
            resolve_aliases(&data.join("Users/tobi/x")),
            PathBuf::from("/Users/tobi/x")
        );
        assert_eq!(resolve_aliases(data), PathBuf::from("/"));
        assert_eq!(
            resolve_aliases(Path::new("/tmp/x")),
            PathBuf::from("/private/tmp/x")
        );

        let home = Path::new("/Users/tobi");
        let via_firmlink = data.join("Users/tobi/Library");
        assert_eq!(
            refuse(&via_firmlink, Path::new("/"), Some(home)),
            refuse(
                Path::new("/Users/tobi/Library"),
                Path::new("/"),
                Some(home)
            )
        );
        assert!(refuse(&via_firmlink, Path::new("/"), Some(home)).is_some());
        assert!(
            refuse(&data.join("Users/tobi"), Path::new("/"), Some(home))
                .is_some_and(|reason| reason.contains("home directory"))
        );
        // The home may itself be spelled through the firmlink.
        let firmlinked_home = data.join("Users/tobi");
        assert!(
            refuse(
                Path::new("/Users/tobi"),
                Path::new("/"),
                Some(&firmlinked_home)
            )
            .is_some_and(|reason| reason.contains("home directory"))
        );
        // Under a whole-disk scan rooted at the Data volume, the firmlinked
        // spelling is still inside the root.
        assert_eq!(
            refuse(Path::new("/Users/tobi/junk"), data, Some(home)),
            None
        );
    }

    #[test]
    fn a_path_inside_a_package_is_refused_but_the_package_is_not() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        let library = root.join("Pics.photoslibrary");
        fs::create_dir_all(library.join("originals")).expect("mkdir");
        fs::write(library.join("originals/1.jpg"), b"jpg").expect("write");
        let app = root.join("Tool.app");
        fs::create_dir_all(app.join("Contents")).expect("mkdir");

        let plan = plan(
            &[
                target(&library.join("originals/1.jpg"), 3),
                target(&app.join("Contents"), 0),
                target(&library, 3),
                target(&app, 0),
            ],
            root,
        );
        assert_eq!(plan.targets.len(), 2, "{plan:?}");
        assert!(plan.targets.iter().any(|target| target.path == library));
        assert!(plan.targets.iter().any(|target| target.path == app));
        assert_eq!(plan.blocked.len(), 2);
        assert!(plan.blocked.iter().all(|blocked| {
            blocked.reason.contains("package")
                && blocked.reason.contains("mark the package itself")
        }));

        // Scanning inside a package looks at its pieces on purpose, except
        // for a Photos library.
        let inside_app = plan_for(&app.join("Contents"), &app);
        assert!(inside_app.blocked.is_empty(), "{inside_app:?}");
        let inside_photos = plan_for(&library.join("originals"), &library);
        assert_eq!(inside_photos.blocked.len(), 1, "{inside_photos:?}");
    }

    fn plan_for(path: &Path, root: &Path) -> Plan {
        plan(&[target(path, 0)], root)
    }

    #[test]
    fn the_home_library_is_refused_but_its_caches_are_not() {
        let home = Path::new("/Users/tobi");
        let root = Path::new("/Users");
        assert!(
            refuse(&home.join("Library"), root, Some(home))
                .is_some_and(|reason| reason.contains("macOS manages this"))
        );
        assert!(
            refuse(&home.join("Library/Keychains"), root, Some(home)).is_some()
        );
        assert!(
            refuse(&home.join("Library/Mobile Documents"), root, Some(home))
                .is_some()
        );
        assert!(refuse(&home.join(".Trash"), root, Some(home)).is_some());
        assert_eq!(
            refuse(&home.join("Library/Caches/foo"), root, Some(home)),
            None
        );
        assert_eq!(
            refuse(
                &home.join("Library/Containers/com.x/Data"),
                root,
                Some(home)
            ),
            None
        );
        assert!(
            refuse(
                Path::new("/Volumes/Disk/.Trashes"),
                Path::new("/"),
                Some(home)
            )
            .is_some_and(|reason| reason.contains("Trash"))
        );
    }

    #[test]
    fn sip_protected_paths_are_refused_by_their_flags() {
        let system_library = Path::new("/System/Library");
        if !system_library.is_dir() {
            return;
        }
        let flags = fs::symlink_metadata(system_library)
            .expect("stat")
            .st_flags();
        assert_ne!(flags & SF_RESTRICTED, 0, "SIP flag on /System/Library");
        let reason = refuse(system_library, Path::new("/"), home().as_deref());
        assert!(reason.is_some_and(|reason| {
            reason.contains("System Integrity Protection")
        }),);
        let users = Path::new("/Users");
        if users.is_dir() {
            let reason = refuse(users, Path::new("/"), home().as_deref());
            assert!(reason.is_some_and(|reason| reason.contains("in place")),);
        }
    }
}
