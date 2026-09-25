//! Free space on the volume a path lives on.
//!
//! This is what makes "amount of crap I found" a real number rather than an
//! estimate: the app measures the volume before and after a removal, and shows
//! the projection while marking.
//!
//! The mount rules themselves live in [`crate::volumes`]; this module is the
//! app-facing surface: capacity, the device to name, and the top of the disk.

use std::io;
use std::path::{Path, PathBuf};

use crate::volumes;

/// A volume's capacity in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpaceInfo {
    /// Total size of the volume.
    pub total: u64,
    /// Free blocks, including the reserve only root may write to.
    pub free: u64,
    /// Free blocks this user may actually write; what `df -h` reports.
    pub available: u64,
    /// Space the system would give back on demand — local Time Machine
    /// snapshots, evicted iCloud files, caches the OS knows it can drop.
    /// Finder counts it as free; `df` does not. Reported separately so the
    /// meter and the before/after comparison keep using `free` and
    /// `available`, the numbers that move when a file is deleted. Zero where
    /// the platform does not report it.
    pub purgeable: u64,
}

impl SpaceInfo {
    /// Space in use, computed from `free` rather than `available` so the figure
    /// does not jump when a reserve is opened to root.
    pub const fn used(&self) -> u64 {
        self.total.saturating_sub(self.free)
    }

    /// Share of the volume in use, `0.0..=1.0`.
    pub fn used_fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.used() as f64 / self.total as f64) as f32
        }
    }

    /// Free space after `bytes` are removed, saturating at the volume size and
    /// never counting the same byte twice.
    #[must_use]
    pub fn after_removing(&self, bytes: u64) -> Self {
        let available = self.available.saturating_add(bytes).min(self.total);
        let free = self.free.saturating_add(bytes).min(self.total);
        Self {
            total: self.total,
            free,
            available,
            purgeable: self.purgeable,
        }
    }
}

/// Read the space on the volume containing `path`.
///
/// `statfs` rather than `statvfs`: on macOS the latter keeps 32-bit counts
/// and saturates on any volume of a few terabytes. APFS volumes in one
/// container share their free space, so the figure is the container's,
/// which is also what Finder shows.
pub fn space_info(path: &Path) -> io::Result<SpaceInfo> {
    let capacity = volumes::capacity_of(path)?;
    let purgeable = purgeable_space(path, capacity.available);
    Ok(SpaceInfo {
        total: capacity.total,
        free: capacity.free,
        available: capacity.available,
        purgeable,
    })
}

/// What the OS would free on demand beyond `available`: Foundation's
/// "available capacity for important usage" includes purgeable space, and
/// `statfs` does not, so the difference is the purgeable amount. Zero when
/// Foundation reports less than `statfs` (they are read at different
/// moments) or cannot answer.
#[cfg(target_os = "macos")]
fn purgeable_space(path: &Path, available: u64) -> u64 {
    use objc2_foundation::{NSArray, NSNumber, NSString, NSURL};

    let Some(url) = NSURL::from_file_path(path) else {
        return 0;
    };
    // Built from its name rather than the exported constant, which would
    // need an `unsafe extern static`.
    let key =
        NSString::from_str("NSURLVolumeAvailableCapacityForImportantUsageKey");
    let keys = NSArray::from_slice(&[&*key]);
    let Ok(values) = url.resourceValuesForKeys_error(&keys) else {
        return 0;
    };
    let Some(value) = values.objectForKey(&key) else {
        return 0;
    };
    value.downcast_ref::<NSNumber>().map_or(0, |number| {
        number.unsignedLongLongValue().saturating_sub(available)
    })
}

#[cfg(not(target_os = "macos"))]
fn purgeable_space(_path: &Path, _available: u64) -> u64 {
    0
}

/// The device a path's filesystem is mounted from, such as `disk3s5`.
///
/// For the boot volume group it is the container with a note, since a scan
/// of `/` covers both of its volumes. `None` where the mount cannot be read.
pub fn device_for(path: &Path) -> Option<String> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let own = volumes::mount_of(&path).ok()?;
    Some(volumes::device_label(&own))
}

/// The top of the disk `path` lives on: `/` for anything in the boot volume
/// group, so `g` and `--disk` from a home directory scan the whole disk, and
/// the volume's mount point for another disk.
pub fn volume_root_for(path: &Path) -> Option<PathBuf> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let own = volumes::mount_of(&path).ok()?;
    Some(volumes::volume_root(&own))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_volume_reports_plausible_numbers() {
        let temp = std::env::temp_dir();
        let space = space_info(&temp).expect("temp dir has a volume");
        assert!(space.total > 0, "{space:?}");
        assert!(space.free <= space.total, "{space:?}");
        assert!(space.available <= space.free, "{space:?}");
        assert!((0.0..=1.0).contains(&space.used_fraction()), "{space:?}");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_home_directory_is_on_the_whole_disk() {
        let home = std::env::var_os("HOME").map(PathBuf::from).expect("HOME");
        assert_eq!(volume_root_for(&home), Some(PathBuf::from("/")));
        let device = device_for(&home).expect("a device");
        assert!(device.starts_with("disk"), "{device}");
        let root = device_for(Path::new("/")).expect("a device");
        assert!(root.ends_with("(boot volume group)"), "{root}");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_root_volume_reports_64_bit_numbers() {
        let space = space_info(Path::new("/")).expect("statfs /");
        // A 32-bit `statvfs` saturates at 4 GiB of blocks; a real disk is
        // bigger than that, so a plausible total proves the 64-bit path.
        assert!(space.total > u64::from(u32::MAX), "{space:?}");
        assert!(space.available <= space.free, "{space:?}");
        assert!(space.free <= space.total, "{space:?}");
        assert!(space.purgeable <= space.total, "{space:?}");
    }

    #[test]
    fn a_missing_path_reports_the_io_error() {
        let error = space_info(Path::new("/definitely/not/here"))
            .expect_err("no volume");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn projecting_removal_cannot_exceed_the_volume() {
        let space = SpaceInfo {
            total: 1000,
            free: 100,
            available: 100,
            purgeable: 7,
        };
        let after = space.after_removing(50);
        assert_eq!(after.available, 150);
        assert_eq!(after.free, 150);
        assert_eq!(after.total, 1000);
        assert_eq!(after.purgeable, 7, "purgeable is not a projection");

        let capped = space.after_removing(10_000);
        assert_eq!(capped.available, 1000);
        assert_eq!(capped.used(), 0);
        assert!((capped.used_fraction() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn used_fraction_handles_an_empty_volume_report() {
        let space = SpaceInfo::default();
        assert!((space.used_fraction() - 0.0).abs() < f32::EPSILON);
    }
}
