//! The mount table, and the rules that decide which mounts a scan enters.
//!
//! On macOS a disk is not one filesystem. The boot disk is an APFS *volume
//! group*: a sealed, read-only system volume mounted at `/` and a data volume
//! mounted at `/System/Volumes/Data`, stitched together by firmlinks so that
//! `/Users`, `/Applications`, `/Library`, `/private`, `/usr/local` and a few
//! more resolve into the data volume. Both volumes report the *same*
//! `st_dev`, so the classic `-x` rule ("stay on the root's device") walks the
//! data volume twice: once through the firmlinks and once under
//! `/System/Volumes/Data`. Volume identity here therefore comes from
//! `statfs`'s `f_fsid`, which does tell the two apart, and the whole-disk
//! scan skips the `/System/Volumes` subtree so the data volume is counted
//! exactly once, through the firmlinks, alongside the sealed system.
//!
//! Everything that decides is a pure function over a [`Mount`] table, so the
//! rules are tested against a fixture of a real machine; only the two
//! functions that read the live table talk to the OS.

use std::path::{Path, PathBuf};

/// `f_flags_ext` bit marking the root data volume of the boot volume group.
/// libc does not expose it; `<sys/mount.h>` defines `MNT_EXT_ROOT_DATA_VOL`
/// as `0x00000001`.
pub const MNT_EXT_ROOT_DATA_VOL: u32 = 0x0000_0001;

/// The mount point of every volume the whole-disk scan must not descend
/// into: the boot volume group's data volume lives here and is already
/// counted through the firmlinks, and its siblings (`Preboot`, `VM`,
/// `Update`, `xarts`, …) are never user data.
const SYSTEM_VOLUMES: &str = "/System/Volumes";

/// `f_flags` bits, from `<sys/mount.h>`, mirrored here so the pure rules do
/// not depend on libc.
const MNT_LOCAL: u32 = 0x0000_1000;
const MNT_ROOTFS: u32 = 0x0000_4000;
const MNT_AUTOMOUNTED: u32 = 0x0040_0000;
/// `MNT_DONTBROWSE`: the Finder hides it, so the volume list does too.
const MNT_DONTBROWSE: u32 = 0x0010_0000;
const MNT_SNAPSHOT: u32 = 0x4000_0000;

/// One mounted filesystem, as `statfs` reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mount {
    /// What is mounted: a device such as `/dev/disk3s5`, or a name like
    /// `devfs` or `map auto_home`.
    pub source: String,
    pub point: PathBuf,
    /// `apfs`, `devfs`, `autofs`, `smbfs`, …
    pub fstype: String,
    /// `f_fsid`, packed: the one identity that tells two APFS volumes of the
    /// same group apart when `st_dev` does not.
    pub fsid: u64,
    /// `f_flags`.
    pub flags: u32,
    /// `f_flags_ext`.
    pub flags_ext: u32,
}

impl Mount {
    /// The sealed system volume, mounted at `/`.
    pub const fn is_root_system(&self) -> bool {
        self.flags & MNT_ROOTFS != 0
    }

    /// The data volume of the boot volume group.
    pub const fn is_root_data(&self) -> bool {
        self.flags_ext & MNT_EXT_ROOT_DATA_VOL != 0
    }

    /// Either half of the boot volume group.
    pub const fn is_boot_group(&self) -> bool {
        self.is_root_system() || self.is_root_data()
    }

    pub const fn is_local(&self) -> bool {
        self.flags & MNT_LOCAL != 0
    }

    /// A snapshot shares its blocks with the live volume; counting it would
    /// count the disk twice.
    pub const fn is_snapshot(&self) -> bool {
        self.flags & MNT_SNAPSHOT != 0
    }

    /// A mount that would be triggered by reading it. Never entered, so a
    /// scan does not mount a NAS just to measure it.
    pub fn is_automount(&self) -> bool {
        self.flags & MNT_AUTOMOUNTED != 0 || self.fstype == "autofs"
    }

    /// Mounted with `nobrowse`: the Finder keeps it off the sidebar and out
    /// of `/Volumes` as shown, so a volume list leaves it out too.
    pub const fn is_hidden(&self) -> bool {
        self.flags & MNT_DONTBROWSE != 0
    }

    /// Mounts that hold no user data under any option: device nodes,
    /// automount triggers, and the boot group's helper volumes (`Preboot`,
    /// `VM`, `Update`, `xarts`, `iSCPreboot`, `Hardware`). The data volume
    /// is the one mount under `/System/Volumes` that is user data.
    pub fn is_never_user_data(&self) -> bool {
        self.fstype == "devfs"
            || self.is_automount()
            || (self.point.starts_with(SYSTEM_VOLUMES) && !self.is_root_data())
    }

    /// `disk3s5` for `/dev/disk3s5`, or the source verbatim when it is not a
    /// device path.
    pub fn device_name(&self) -> &str {
        self.source
            .strip_prefix("/dev/")
            .unwrap_or(self.source.as_str())
    }

    /// The physical container of an APFS volume: `disk3` for `disk3s1s1`.
    fn container_name(&self) -> &str {
        let device = self.device_name();
        let digits = device.strip_prefix("disk").map_or(0, |rest| {
            rest.bytes().take_while(u8::is_ascii_digit).count()
        });
        if digits == 0 {
            device
        } else {
            &device[.."disk".len() + digits]
        }
    }
}

/// The mount `path` lives on: the one with the longest mount point above it.
///
/// Firmlinks are invisible to this: `/Users/x` matches `/`, not the data
/// volume at `/System/Volumes/Data`. The live functions use `statfs`, which
/// resolves them; this is for the fixture-driven rules, where either half of
/// the boot group gives the same answer.
pub fn mount_containing<'a>(
    mounts: &'a [Mount],
    path: &Path,
) -> Option<&'a Mount> {
    mounts
        .iter()
        .filter(|mount| path.starts_with(&mount.point))
        .max_by_key(|mount| mount.point.as_os_str().len())
}

/// The mounts a scan rooted on `own` may enter: `own` itself, plus both
/// halves of the boot volume group when `own` is one of them.
pub fn volume_group<'a>(mounts: &'a [Mount], own: &'a Mount) -> Vec<&'a Mount> {
    mounts
        .iter()
        .filter(|mount| {
            mount.fsid == own.fsid
                || (own.is_boot_group() && mount.is_boot_group())
        })
        .collect()
}

/// The top of the disk `own` belongs to: `/` for either half of the boot
/// volume group, since the whole-disk scan covers both, and the mount point
/// itself for any other volume.
pub fn volume_root(own: &Mount) -> PathBuf {
    if own.is_boot_group() {
        PathBuf::from("/")
    } else {
        own.point.clone()
    }
}

/// What the disk section names: the container for the boot volume group,
/// since a scan of `/` covers both of its volumes, and the volume's device
/// otherwise.
pub fn device_label(own: &Mount) -> String {
    if own.is_root_system() {
        format!("{} (boot volume group)", own.container_name())
    } else {
        own.device_name().to_string()
    }
}

/// A volume the user could pick to scan: what the Finder shows under
/// "Locations", one row for the whole boot volume group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    /// The mount point's own name; `/` for the boot disk until the caller
    /// looks its Finder name up, see [`boot_volume_name`].
    pub name: String,
    /// What [`volume_root`] gives: `/` for the boot group.
    pub root: PathBuf,
    pub is_boot: bool,
}

/// The volumes worth offering as a scan root, the boot disk first and the
/// rest in mount order.
///
/// The boot volume group is one entry at `/`; hidden (`nobrowse`) mounts,
/// snapshots, device nodes and automount triggers are left out, since the
/// Finder shows none of them either and an automount would be created by
/// scanning it. A mounted network share is listed: it is already mounted,
/// so measuring it creates nothing.
pub fn volumes_to_scan(mounts: &[Mount]) -> Vec<Volume> {
    let mut list: Vec<Volume> = Vec::new();
    for mount in mounts {
        // The sealed system volume is itself mounted as a snapshot; the
        // snapshot rule is for Time Machine's local ones, not for it.
        let snapshot = mount.is_snapshot() && !mount.is_boot_group();
        if mount.is_never_user_data()
            || snapshot
            || mount.is_hidden()
            || mount.is_root_data()
        {
            continue;
        }
        let root = volume_root(mount);
        if list.iter().any(|volume| volume.root == root) {
            continue;
        }
        let name = root.file_name().map_or_else(
            || root.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        list.push(Volume {
            name,
            root,
            is_boot: mount.is_boot_group(),
        });
    }
    list.sort_by_key(|volume| !volume.is_boot);
    list
}

/// The Finder's name for the boot disk ("Macintosh HD" unless renamed):
/// the entry of `/Volumes` that is a link to `/`. `None` when there is
/// none to read, and the caller shows `/`.
pub fn boot_volume_name() -> Option<String> {
    let entries = std::fs::read_dir("/Volumes").ok()?;
    entries
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .path()
                .read_link()
                .is_ok_and(|target| target == Path::new("/"))
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
}

/// [`volumes_to_scan`] for this machine, the boot disk under its Finder
/// name.
pub fn scannable_volumes() -> Vec<Volume> {
    let mut list = volumes_to_scan(&mount_table());
    if let Some(name) = boot_volume_name() {
        for volume in list.iter_mut().filter(|volume| volume.is_boot) {
            volume.name.clone_from(&name);
        }
    }
    list
}

/// Mount points strictly below `root` that a scan must not enter, where
/// `own` is the mount `root` lives on.
///
/// With `one_filesystem`, the scan stays on `own`'s volume group: other
/// disks, other APFS volumes of the same container, network shares,
/// snapshots and automount triggers are left out, and when `root` is on the
/// sealed system volume the whole `/System/Volumes` subtree is skipped so the
/// data volume is counted once, through the firmlinks. Without it the scan
/// crosses into everything that can hold user data, which still leaves out
/// `/dev`, automount triggers and the boot group's helper volumes: they are
/// never anyone's files, and an automount would be *created* by measuring
/// it.
///
/// Every path is listed in both spellings a firmlinked mount can have —
/// `/Users/x/…` and `/System/Volumes/Data/Users/x/…` — so the check holds
/// whichever form the table reports and the walk reaches.
pub fn excluded_mounts(
    mounts: &[Mount],
    own: &Mount,
    root: &Path,
    one_filesystem: bool,
) -> Vec<PathBuf> {
    let data_point = mounts
        .iter()
        .find(|mount| mount.is_root_data())
        .map(|mount| mount.point.clone());
    let group: Vec<u64> = volume_group(mounts, own)
        .iter()
        .map(|mount| mount.fsid)
        .collect();
    let mut excluded: Vec<PathBuf> = mounts
        .iter()
        .filter(|mount| mount.point != root && mount.point.starts_with(root))
        .filter(|mount| {
            mount.is_never_user_data()
                || (one_filesystem
                    && (!group.contains(&mount.fsid)
                        || !mount.is_local()
                        || mount.is_snapshot()))
        })
        .flat_map(|mount| spellings(&mount.point, data_point.as_deref()))
        .collect();
    let system_volumes = Path::new(SYSTEM_VOLUMES);
    if one_filesystem
        && own.is_root_system()
        && system_volumes != root
        && system_volumes.starts_with(root)
    {
        excluded.push(system_volumes.to_path_buf());
    }
    excluded.sort();
    excluded.dedup();
    excluded
}

/// A mount point and its other spelling across the root data firmlinks.
fn spellings(point: &Path, data_point: Option<&Path>) -> Vec<PathBuf> {
    let mut forms = vec![point.to_path_buf()];
    if let Some(data) = data_point {
        if let Ok(below) = point.strip_prefix(data) {
            if !below.as_os_str().is_empty() {
                forms.push(Path::new("/").join(below));
            }
        } else if let Ok(below) = point.strip_prefix("/")
            && !below.as_os_str().is_empty()
            && !point.starts_with(SYSTEM_VOLUMES)
        {
            forms.push(data.join(below));
        }
    }
    forms
}

/// Free space as `statfs` reports it, in bytes. `statvfs` on macOS keeps
/// 32-bit counts and saturates on a large volume; `statfs` is 64-bit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capacity {
    pub total: u64,
    /// Including the reserve only root may write to.
    pub free: u64,
    /// What this user may write; what `df -h` shows.
    pub available: u64,
}

#[cfg(target_os = "macos")]
mod live {
    use std::ffi::CString;
    use std::io;
    use std::path::{Path, PathBuf};

    use super::{Capacity, Mount};

    fn name(field: &[libc::c_char]) -> String {
        // The kernel NUL-terminates these; the fallback covers a name that
        // fills the whole field.
        let bytes: Vec<u8> = field
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// `f_fsid` packed into one number. libc keeps `fsid_t`'s two words
    /// private, and the type is `repr(C)` of exactly two `i32`s, which the
    /// transmute checks by size.
    #[allow(unsafe_code, reason = "libc::fsid_t has no accessor")]
    fn fsid(fsid: libc::fsid_t) -> u64 {
        // SAFETY: `fsid_t` is `#[repr(C)] { val: [i32; 2] }` in
        // <sys/types.h>; any bit pattern is a valid `[i32; 2]`.
        let [hi, lo]: [i32; 2] = unsafe { std::mem::transmute(fsid) };
        (u64::from(hi as u32) << 32) | u64::from(lo as u32)
    }

    fn mount_from(stat: &libc::statfs) -> Mount {
        Mount {
            source: name(&stat.f_mntfromname),
            point: PathBuf::from(name(&stat.f_mntonname)),
            fstype: name(&stat.f_fstypename),
            fsid: fsid(stat.f_fsid),
            flags: stat.f_flags,
            flags_ext: stat.f_flags_ext,
        }
    }

    fn capacity_from(stat: &libc::statfs) -> Capacity {
        let block = u64::from(stat.f_bsize);
        Capacity {
            total: stat.f_blocks.saturating_mul(block),
            free: stat.f_bfree.saturating_mul(block),
            available: stat.f_bavail.saturating_mul(block),
        }
    }

    /// `statfs(2)` for `path`.
    #[allow(unsafe_code, reason = "statfs(2) has no safe 64-bit wrapper")]
    fn statfs(path: &Path) -> io::Result<libc::statfs> {
        use std::os::unix::ffi::OsStrExt as _;
        let path = CString::new(path.as_os_str().as_bytes())?;
        let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: `path` is NUL-terminated and `stat` is a writable buffer of
        // the size statfs expects; it is only read once the call reports
        // success, when the kernel has filled it in.
        let code = unsafe { libc::statfs(path.as_ptr(), stat.as_mut_ptr()) };
        if code != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: statfs returned 0, so the buffer is initialised.
        Ok(unsafe { stat.assume_init() })
    }

    /// The filesystem `path` lives on, resolving firmlinks: `/Users/x` is
    /// reported on the data volume, not on `/`.
    pub fn mount_of(path: &Path) -> io::Result<Mount> {
        statfs(path).map(|stat| mount_from(&stat))
    }

    pub fn capacity_of(path: &Path) -> io::Result<Capacity> {
        statfs(path).map(|stat| capacity_from(&stat))
    }

    /// Every mounted filesystem, from `getfsstat(2)` with `MNT_NOWAIT`: the
    /// cached table, so no filesystem is asked to refresh itself, which for a
    /// dead network share would hang.
    #[allow(unsafe_code, reason = "getfsstat(2) has no safe wrapper")]
    pub fn mount_table() -> Vec<Mount> {
        // SAFETY: a null buffer asks only for the count; nothing is written.
        let count = unsafe {
            libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT)
        };
        let Ok(count) = usize::try_from(count) else {
            return Vec::new();
        };
        // Mounts may appear between the two calls; the second call is told
        // the real buffer size, so extra ones are simply left out.
        let mut buffer: Vec<libc::statfs> = Vec::with_capacity(count);
        let size = count.saturating_mul(std::mem::size_of::<libc::statfs>());
        let Ok(size) = libc::c_int::try_from(size) else {
            return Vec::new();
        };
        // SAFETY: the buffer has capacity for `count` entries and `size` is
        // exactly that many bytes; the kernel returns how many it filled.
        let filled = unsafe {
            libc::getfsstat(buffer.as_mut_ptr(), size, libc::MNT_NOWAIT)
        };
        let Ok(filled) = usize::try_from(filled) else {
            return Vec::new();
        };
        // SAFETY: getfsstat initialised the first `filled` entries, and
        // `filled <= count` by the size it was given.
        unsafe { buffer.set_len(filled.min(count)) };
        buffer.iter().map(mount_from).collect()
    }
}

#[cfg(not(target_os = "macos"))]
mod live {
    use std::io;
    use std::path::Path;

    use super::{Capacity, Mount};

    /// No mount table off macOS: callers fall back to device comparison.
    pub fn mount_table() -> Vec<Mount> {
        Vec::new()
    }

    pub fn mount_of(_path: &Path) -> io::Result<Mount> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    pub fn capacity_of(path: &Path) -> io::Result<Capacity> {
        let stat = rustix::fs::statvfs(path)?;
        // `f_frsize` is the fragment size the counts are expressed in; some
        // filesystems report zero for it, so fall back to `f_bsize`.
        let block = if stat.f_frsize == 0 {
            stat.f_bsize
        } else {
            stat.f_frsize
        };
        Ok(Capacity {
            total: stat.f_blocks.saturating_mul(block),
            free: stat.f_bfree.saturating_mul(block),
            available: stat.f_bavail.saturating_mul(block),
        })
    }
}

pub use live::{capacity_of, mount_of, mount_table};

/// [`excluded_mounts`] for this machine, with `root` in the form the walk
/// uses. `None` when the mount `root` lives on cannot be read, so the caller
/// can fall back to comparing devices.
pub fn excluded_mounts_for(
    root: &Path,
    one_filesystem: bool,
) -> Option<Vec<PathBuf>> {
    let own = mount_of(root).ok()?;
    Some(excluded_mounts(&mount_table(), &own, root, one_filesystem))
}

/// The mount points of the volume group `root` lives on, for the device
/// backstop: a directory whose device is not one of theirs is a mount the
/// table did not list.
pub fn volume_group_for(root: &Path) -> Vec<PathBuf> {
    let Ok(own) = mount_of(root) else {
        return Vec::new();
    };
    let table = mount_table();
    volume_group(&table, &own)
        .into_iter()
        .map(|mount| mount.point.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const APFS: &str = "apfs";

    /// This machine's table on macOS 27.2, SIP on: a sealed system volume, a
    /// data volume, the helper volumes, a second user volume in the same
    /// container, the simulator runtime, the `CoreDevice` file system and the
    /// autofs trigger for `/home`.
    fn fixture() -> Vec<Mount> {
        let local = MNT_LOCAL;
        let nobrowse = 0x0010_0000;
        let rdonly = 0x1;
        let mount = |source: &str,
                     point: &str,
                     fstype: &str,
                     fsid: u64,
                     flags: u32,
                     ext: u32| Mount {
            source: source.into(),
            point: PathBuf::from(point),
            fstype: fstype.into(),
            fsid,
            flags,
            flags_ext: ext,
        };
        vec![
            mount(
                "/dev/disk3s1s1",
                "/",
                APFS,
                16_777_236,
                local | MNT_ROOTFS | rdonly,
                0,
            ),
            mount("devfs", "/dev", "devfs", 2, local | nobrowse, 0),
            mount(
                "/dev/disk3s6",
                "/System/Volumes/VM",
                APFS,
                16_777_240,
                local | nobrowse,
                0,
            ),
            mount(
                "/dev/disk3s2",
                "/System/Volumes/Preboot",
                APFS,
                16_777_241,
                local | nobrowse,
                0,
            ),
            mount(
                "/dev/disk3s4",
                "/System/Volumes/Update",
                APFS,
                16_777_242,
                local | nobrowse,
                0,
            ),
            mount(
                "/dev/disk1s2",
                "/System/Volumes/xarts",
                APFS,
                16_777_220,
                local | nobrowse,
                0,
            ),
            mount(
                "/dev/disk1s1",
                "/System/Volumes/iSCPreboot",
                APFS,
                16_777_221,
                local | nobrowse,
                0,
            ),
            mount(
                "/dev/disk1s3",
                "/System/Volumes/Hardware",
                APFS,
                16_777_222,
                local | nobrowse,
                0,
            ),
            mount(
                "/dev/disk3s5",
                "/System/Volumes/Data",
                APFS,
                16_777_232,
                local | nobrowse,
                MNT_EXT_ROOT_DATA_VOL,
            ),
            mount("/dev/disk3s7", "/Volumes/Space", APFS, 16_777_250, local, 0),
            mount(
                "map auto_home",
                "/System/Volumes/Data/home",
                "autofs",
                3,
                MNT_AUTOMOUNTED | nobrowse,
                0,
            ),
            mount(
                "/dev/disk3s3",
                "/Volumes/Recovery",
                APFS,
                16_777_243,
                local | nobrowse,
                0,
            ),
            mount(
                "/dev/disk5s1",
                "/Library/Developer/CoreSimulator/Volumes/iOS_24A434",
                APFS,
                16_777_260,
                local | nobrowse | rdonly,
                0,
            ),
            mount(
                "devices",
                "/Users/Adre/Library/Developer/CoreDevice/DeviceFS",
                "devicefs",
                4,
                local | nobrowse,
                0,
            ),
            mount("//nas/share", "/Users/Adre/nas", "smbfs", 5, 0, 0),
            mount(
                "/dev/disk3s5",
                "/Volumes/com.apple.TimeMachine.localsnapshots/Backups.backupdb/x/2026-09-25/Data",
                APFS,
                16_777_270,
                local | MNT_SNAPSHOT | rdonly,
                0,
            ),
        ]
    }

    fn named<'a>(mounts: &'a [Mount], point: &str) -> &'a Mount {
        mounts
            .iter()
            .find(|mount| mount.point == Path::new(point))
            .expect("fixture mount")
    }

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_whole_disk_skips_system_volumes_and_every_other_mount() {
        let mounts = fixture();
        let own = named(&mounts, "/");
        let excluded = excluded_mounts(&mounts, own, Path::new("/"), true);
        assert!(
            excluded.contains(&PathBuf::from("/System/Volumes")),
            "{excluded:?}"
        );
        for point in [
            "/dev",
            "/Volumes/Space",
            "/Volumes/Recovery",
            "/System/Volumes/Data/home",
            "/home",
            "/Library/Developer/CoreSimulator/Volumes/iOS_24A434",
            "/Users/Adre/Library/Developer/CoreDevice/DeviceFS",
            "/Users/Adre/nas",
        ] {
            assert!(
                excluded.contains(&PathBuf::from(point)),
                "{point} in {excluded:?}"
            );
        }
        assert!(!excluded.contains(&PathBuf::from("/")));
        assert!(
            !excluded.iter().any(|path| path == Path::new("/Users")),
            "the firmlinks into the data volume are walked"
        );
    }

    #[test]
    fn the_data_volume_is_counted_once_through_the_firmlinks() {
        let mounts = fixture();
        let own = named(&mounts, "/");
        let excluded = excluded_mounts(&mounts, own, Path::new("/"), true);
        // Under `/`, the only way into the data volume must be the firmlinks:
        // its mount point is under the skipped subtree, and nothing above it
        // is in the list.
        assert!(excluded.contains(&PathBuf::from("/System/Volumes")));
        assert!(!excluded.contains(&PathBuf::from("/System")));
    }

    #[test]
    fn a_home_scan_stays_in_the_boot_volume_group() {
        let mounts = fixture();
        let own = named(&mounts, "/System/Volumes/Data");
        let home = Path::new("/Users/Adre");
        let excluded = excluded_mounts(&mounts, own, home, true);
        assert_eq!(
            excluded,
            paths(&[
                "/System/Volumes/Data/Users/Adre/Library/Developer/CoreDevice/DeviceFS",
                "/System/Volumes/Data/Users/Adre/nas",
                "/Users/Adre/Library/Developer/CoreDevice/DeviceFS",
                "/Users/Adre/nas",
            ]),
            "both spellings of every foreign mount under the home"
        );
    }

    #[test]
    fn a_volume_root_is_its_own_group() {
        let mounts = fixture();
        let own = named(&mounts, "/Volumes/Space");
        let group: Vec<&Path> = volume_group(&mounts, own)
            .iter()
            .map(|mount| mount.point.as_path())
            .collect();
        assert_eq!(group, [Path::new("/Volumes/Space")]);
        let excluded = excluded_mounts(
            &mounts,
            own,
            Path::new("/Volumes/Space/foo"),
            true,
        );
        assert!(excluded.is_empty(), "{excluded:?}");
        assert_eq!(volume_root(own), PathBuf::from("/Volumes/Space"));
    }

    #[test]
    fn the_boot_group_spans_both_halves() {
        let mounts = fixture();
        let data = named(&mounts, "/System/Volumes/Data");
        let mut group: Vec<&Path> = volume_group(&mounts, data)
            .iter()
            .map(|mount| mount.point.as_path())
            .collect();
        group.sort();
        assert_eq!(group, [Path::new("/"), Path::new("/System/Volumes/Data")]);
        assert_eq!(volume_root(data), PathBuf::from("/"));
        assert_eq!(volume_root(named(&mounts, "/")), PathBuf::from("/"));
    }

    #[test]
    fn automount_triggers_are_excluded_under_every_option() {
        let mounts = fixture();
        let own = named(&mounts, "/");
        let crossing = excluded_mounts(&mounts, own, Path::new("/"), false);
        for point in [
            "/System/Volumes/Data/home",
            "/home",
            "/dev",
            "/System/Volumes/VM",
            "/System/Volumes/Preboot",
            "/System/Volumes/Update",
            "/System/Volumes/xarts",
            "/System/Volumes/iSCPreboot",
            "/System/Volumes/Hardware",
        ] {
            assert!(
                crossing.contains(&PathBuf::from(point)),
                "{point} in {crossing:?}"
            );
        }
        for point in ["/System/Volumes", "/Volumes/Space", "/Users/Adre/nas"] {
            assert!(
                !crossing.contains(&PathBuf::from(point)),
                "-X crosses into {point}"
            );
        }
    }

    #[test]
    fn snapshots_and_network_shares_are_left_out() {
        let mounts = fixture();
        let own = named(&mounts, "/System/Volumes/Data");
        let excluded =
            excluded_mounts(&mounts, own, Path::new("/Volumes"), true);
        assert!(excluded.iter().any(|path| {
            path.starts_with("/Volumes/com.apple.TimeMachine.localsnapshots")
        }));
        let excluded = excluded_mounts(&mounts, own, Path::new("/Users"), true);
        assert!(excluded.contains(&PathBuf::from("/Users/Adre/nas")));
    }

    #[test]
    fn the_volume_list_is_what_the_finder_shows() {
        let list = volumes_to_scan(&fixture());
        let roots: Vec<&Path> =
            list.iter().map(|volume| volume.root.as_path()).collect();
        // The boot group once, first; the external volume; the mounted
        // share; and nothing hidden, helper, snapshot, automount or device.
        assert_eq!(
            roots,
            [
                Path::new("/"),
                Path::new("/Volumes/Space"),
                Path::new("/Users/Adre/nas")
            ]
        );
        assert!(list[0].is_boot);
        assert_eq!(list[0].name, "/");
        assert_eq!(list[1].name, "Space");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn this_machine_lists_its_boot_disk_by_name() {
        let list = scannable_volumes();
        let boot = list.iter().find(|volume| volume.is_boot).expect("boot");
        assert_eq!(boot.root, Path::new("/"));
        assert_ne!(boot.name, "/", "{list:?}");
    }

    #[test]
    fn the_device_label_names_the_container_for_the_boot_group() {
        let mounts = fixture();
        assert_eq!(
            device_label(named(&mounts, "/")),
            "disk3 (boot volume group)"
        );
        assert_eq!(
            device_label(named(&mounts, "/System/Volumes/Data")),
            "disk3s5"
        );
        assert_eq!(device_label(named(&mounts, "/Volumes/Space")), "disk3s7");
        assert_eq!(device_label(named(&mounts, "/dev")), "devfs");
    }

    #[test]
    fn the_containing_mount_is_the_longest_prefix() {
        let mounts = fixture();
        let own = mount_containing(&mounts, Path::new("/Volumes/Space/x"));
        assert_eq!(
            own.map(|mount| mount.point.as_path()),
            Some(Path::new("/Volumes/Space"))
        );
        let own = mount_containing(&mounts, Path::new("/usr/bin"));
        assert_eq!(
            own.map(|mount| mount.point.as_path()),
            Some(Path::new("/"))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn this_machine_has_a_boot_volume_group() {
        let table = mount_table();
        assert!(table.iter().any(Mount::is_root_system), "{table:?}");
        let root = mount_of(Path::new("/")).expect("statfs /");
        assert!(root.is_root_system(), "{root:?}");
        let home = std::env::var_os("HOME").map(PathBuf::from).expect("HOME");
        let own = mount_of(&home).expect("statfs home");
        assert!(own.is_boot_group(), "{own:?}");
        assert_eq!(volume_root(&own), PathBuf::from("/"));
        let capacity = capacity_of(Path::new("/")).expect("statfs /");
        assert!(capacity.total > u64::from(u32::MAX), "{capacity:?}");
        assert!(capacity.available <= capacity.free);
        assert!(capacity.free <= capacity.total);
    }
}
