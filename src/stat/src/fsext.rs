// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

pub use super::uucore::libc;
extern crate time;

use self::time::Timespec;
pub use libc::{
    c_int, mode_t, strerror, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG,
    S_IFSOCK, S_IRGRP, S_IROTH, S_IRUSR, S_ISGID, S_ISUID, S_ISVTX, S_IWGRP, S_IWOTH, S_IWUSR,
    S_IXGRP, S_IXOTH, S_IXUSR,
};
use std::time::UNIX_EPOCH;

pub trait BirthTime {
    fn pretty_birth(&self) -> String;
    fn birth(&self) -> String;
}

use std::fs::Metadata;
impl BirthTime for Metadata {
    fn pretty_birth(&self) -> String {
        self.created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|e| pretty_time(e.as_secs() as i64, i64::from(e.subsec_nanos())))
            .unwrap_or_else(|| "-".to_owned())
    }

    fn birth(&self) -> String {
        self.created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|e| format!("{}", e.as_secs()))
            .unwrap_or_else(|| "0".to_owned())
    }
}

#[macro_export]
macro_rules! has {
    ($mode:expr, $perm:expr) => {
        $mode & $perm != 0
    };
}

pub fn pretty_time(sec: i64, nsec: i64) -> String {
    // sec == seconds since UNIX_EPOCH
    // nsec == nanoseconds since (UNIX_EPOCH + sec)
    let tm = time::at(Timespec::new(sec, nsec as i32));
    let res = time::strftime("%Y-%m-%d %H:%M:%S.%f %z", &tm).unwrap();
    if res.ends_with(" -0000") {
        res.replace(" -0000", " +0000")
    } else {
        res
    }
}

pub fn pretty_filetype<'a>(mode: mode_t, size: u64) -> &'a str {
    match mode & S_IFMT {
        S_IFREG => {
            if size != 0 {
                "regular file"
            } else {
                "regular empty file"
            }
        }
        S_IFDIR => "directory",
        S_IFLNK => "symbolic link",
        S_IFCHR => "character special file",
        S_IFBLK => "block special file",
        S_IFIFO => "fifo",
        S_IFSOCK => "socket",
        // TODO: Other file types
        // See coreutils/gnulib/lib/file-type.c
        _ => "weird file",
    }
}

pub fn pretty_access(mode: mode_t) -> String {
    let mut result = String::with_capacity(10);
    result.push(match mode & S_IFMT {
        S_IFDIR => 'd',
        S_IFCHR => 'c',
        S_IFBLK => 'b',
        S_IFREG => '-',
        S_IFIFO => 'p',
        S_IFLNK => 'l',
        S_IFSOCK => 's',
        // TODO: Other file types
        _ => '?',
    });

    result.push(if has!(mode, S_IRUSR) { 'r' } else { '-' });
    result.push(if has!(mode, S_IWUSR) { 'w' } else { '-' });
    result.push(if has!(mode, S_ISUID as mode_t) {
        if has!(mode, S_IXUSR) {
            's'
        } else {
            'S'
        }
    } else if has!(mode, S_IXUSR) {
        'x'
    } else {
        '-'
    });

    result.push(if has!(mode, S_IRGRP) { 'r' } else { '-' });
    result.push(if has!(mode, S_IWGRP) { 'w' } else { '-' });
    result.push(if has!(mode, S_ISGID as mode_t) {
        if has!(mode, S_IXGRP) {
            's'
        } else {
            'S'
        }
    } else if has!(mode, S_IXGRP) {
        'x'
    } else {
        '-'
    });

    result.push(if has!(mode, S_IROTH) { 'r' } else { '-' });
    result.push(if has!(mode, S_IWOTH) { 'w' } else { '-' });
    result.push(if has!(mode, S_ISVTX as mode_t) {
        if has!(mode, S_IXOTH) {
            't'
        } else {
            'T'
        }
    } else if has!(mode, S_IXOTH) {
        'x'
    } else {
        '-'
    });

    result
}

use std::borrow::Cow;
use std::convert::{AsRef, From};
use std::ffi::CString;
use std::io::Error as IOError;
use std::mem;
use std::path::Path;

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "freebsd"
))]
use libc::statfs as Sstatfs;
#[cfg(any(
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "bitrig",
    target_os = "dragonfly"
))]
use libc::statvfs as Sstatfs;

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "freebsd"
))]
use libc::statfs as statfs_fn;
#[cfg(any(
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "bitrig",
    target_os = "dragonfly"
))]
use libc::statvfs as statfs_fn;

pub trait FsMeta {
    fn fs_type(&self) -> i64;
    fn iosize(&self) -> u64;
    fn blksize(&self) -> i64;
    fn total_blocks(&self) -> u64;
    fn free_blocks(&self) -> u64;
    fn avail_blocks(&self) -> u64;
    fn total_fnodes(&self) -> u64;
    fn free_fnodes(&self) -> u64;
    fn fsid(&self) -> u64;
    fn namelen(&self) -> u64;
}

impl FsMeta for Sstatfs {
    fn blksize(&self) -> i64 {
        self.f_bsize as i64
    }
    fn total_blocks(&self) -> u64 {
        self.f_blocks as u64
    }
    fn free_blocks(&self) -> u64 {
        self.f_bfree as u64
    }
    fn avail_blocks(&self) -> u64 {
        self.f_bavail as u64
    }
    fn total_fnodes(&self) -> u64 {
        self.f_files as u64
    }
    fn free_fnodes(&self) -> u64 {
        self.f_ffree as u64
    }
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))]
    fn fs_type(&self) -> i64 {
        self.f_type as i64
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
    fn fs_type(&self) -> i64 {
        // FIXME: statvfs doesn't have an equivalent, so we need to do something else
        unimplemented!()
    }

    #[cfg(target_os = "linux")]
    fn iosize(&self) -> u64 {
        self.f_frsize as u64
    }
    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    fn iosize(&self) -> u64 {
        self.f_iosize as u64
    }
    // XXX: dunno if this is right
    #[cfg(not(any(target_os = "macos", target_os = "freebsd", target_os = "linux")))]
    fn iosize(&self) -> u64 {
        self.f_bsize as u64
    }

    // Linux, SunOS, HP-UX, 4.4BSD, FreeBSD have a system call statfs() that returns
    // a struct statfs, containing a fsid_t f_fsid, where fsid_t is defined
    // as struct { int val[2];  }
    //
    // Solaris, Irix and POSIX have a system call statvfs(2) that returns a
    // struct statvfs, containing an  unsigned  long  f_fsid
    #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "linux"))]
    fn fsid(&self) -> u64 {
        let f_fsid: &[u32; 2] =
            unsafe { &*(&self.f_fsid as *const uucore::libc::fsid_t as *const [u32; 2]) };
        (u64::from(f_fsid[0])) << 32 | u64::from(f_fsid[1])
    }
    #[cfg(not(any(target_os = "macos", target_os = "freebsd", target_os = "linux")))]
    fn fsid(&self) -> u64 {
        self.f_fsid as u64
    }

    #[cfg(target_os = "linux")]
    fn namelen(&self) -> u64 {
        self.f_namelen as u64
    }
    #[cfg(target_os = "macos")]
    fn namelen(&self) -> u64 {
        1024
    }
    #[cfg(target_os = "freebsd")]
    fn namelen(&self) -> u64 {
        self.f_namemax as u64
    }
    // XXX: should everything just use statvfs?
    #[cfg(not(any(target_os = "macos", target_os = "freebsd", target_os = "linux")))]
    fn namelen(&self) -> u64 {
        self.f_namemax as u64
    }
}

pub fn statfs<P: AsRef<Path>>(path: P) -> Result<Sstatfs, String>
where
    Vec<u8>: From<P>,
{
    match CString::new(path) {
        Ok(p) => {
            let mut buffer: Sstatfs = unsafe { mem::zeroed() };
            unsafe {
                match statfs_fn(p.as_ptr(), &mut buffer) {
                    0 => Ok(buffer),
                    _ => {
                        let errno = IOError::last_os_error().raw_os_error().unwrap_or(0);
                        Err(CString::from_raw(strerror(errno))
                            .into_string()
                            .unwrap_or_else(|_| "Unknown Error".to_owned()))
                    }
                }
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn pretty_fstype<'a>(fstype: i64) -> Cow<'a, str> {
    match fstype {
        0x6163_6673 => "acfs".into(),
        0xADF5 => "adfs".into(),
        0xADFF => "affs".into(),
        0x5346_414F => "afs".into(),
        0x0904_1934 => "anon-inode FS".into(),
        0x6175_6673 => "aufs".into(),
        0x0187 => "autofs".into(),
        0x4246_5331 => "befs".into(),
        0x6264_6576 => "bdevfs".into(),
        0x1BAD_FACE => "bfs".into(),
        0xCAFE_4A11 => "bpf_fs".into(),
        0x4249_4E4D => "binfmt_misc".into(),
        0x9123_683E => "btrfs".into(),
        0x7372_7279 => "btrfs_test".into(),
        0x00C3_6400 => "ceph".into(),
        0x0027_E0EB => "cgroupfs".into(),
        0xFF53_4D42 => "cifs".into(),
        0x7375_7245 => "coda".into(),
        0x012F_F7B7 => "coh".into(),
        0x6265_6570 => "configfs".into(),
        0x28CD_3D45 => "cramfs".into(),
        0x453D_CD28 => "cramfs-wend".into(),
        0x6462_6720 => "debugfs".into(),
        0x1373 => "devfs".into(),
        0x1CD1 => "devpts".into(),
        0xF15F => "ecryptfs".into(),
        0xDE5E_81E4 => "efivarfs".into(),
        0x0041_4A53 => "efs".into(),
        0x5DF5 => "exofs".into(),
        0x137D => "ext".into(),
        0xEF53 => "ext2/ext3".into(),
        0xEF51 => "ext2".into(),
        0xF2F5_2010 => "f2fs".into(),
        0x4006 => "fat".into(),
        0x1983_0326 => "fhgfs".into(),
        0x6573_5546 => "fuseblk".into(),
        0x6573_5543 => "fusectl".into(),
        0x0BAD_1DEA => "futexfs".into(),
        0x0116_1970 => "gfs/gfs2".into(),
        0x4750_4653 => "gpfs".into(),
        0x4244 => "hfs".into(),
        0x482B => "hfs+".into(),
        0x4858 => "hfsx".into(),
        0x00C0_FFEE => "hostfs".into(),
        0xF995_E849 => "hpfs".into(),
        0x9584_58F6 => "hugetlbfs".into(),
        0x1130_7854 => "inodefs".into(),
        0x0131_11A8 => "ibrix".into(),
        0x2BAD_1DEA => "inotifyfs".into(),
        0x9660 => "isofs".into(),
        0x4004 => "isofs".into(),
        0x4000 => "isofs".into(),
        0x07C0 => "jffs".into(),
        0x72B6 => "jffs2".into(),
        0x3153_464A => "jfs".into(),
        0x6B41_4653 => "k-afs".into(),
        0xC97E_8168 => "logfs".into(),
        0x0BD0_0BD0 => "lustre".into(),
        0x5346_314D => "m1fs".into(),
        0x137F => "minix".into(),
        0x138F => "minix (30 char.)".into(),
        0x2468 => "minix v2".into(),
        0x2478 => "minix v2 (30 char.)".into(),
        0x4D5A => "minix3".into(),
        0x1980_0202 => "mqueue".into(),
        0x4D44 => "msdos".into(),
        0x564C => "novell".into(),
        0x6969 => "nfs".into(),
        0x6E66_7364 => "nfsd".into(),
        0x3434 => "nilfs".into(),
        0x6E73_6673 => "nsfs".into(),
        0x5346_544E => "ntfs".into(),
        0x9FA1 => "openprom".into(),
        0x7461_636F => "ocfs2".into(),
        0x794C_7630 => "overlayfs".into(),
        0xAAD7_AAEA => "panfs".into(),
        0x5049_5045 => "pipefs".into(),
        0x7C7C_6673 => "prl_fs".into(),
        0x9FA0 => "proc".into(),
        0x6165_676C => "pstorefs".into(),
        0x002F => "qnx4".into(),
        0x6819_1122 => "qnx6".into(),
        0x8584_58F6 => "ramfs".into(),
        0x5265_4973 => "reiserfs".into(),
        0x7275 => "romfs".into(),
        0x6759_6969 => "rpc_pipefs".into(),
        0x7363_6673 => "securityfs".into(),
        0xF97C_FF8C => "selinux".into(),
        0x4341_5D53 => "smackfs".into(),
        0x517B => "smb".into(),
        0xFE53_4D42 => "smb2".into(),
        0xBEEF_DEAD => "snfs".into(),
        0x534F_434B => "sockfs".into(),
        0x7371_7368 => "squashfs".into(),
        0x6265_6572 => "sysfs".into(),
        0x012F_F7B6 => "sysv2".into(),
        0x012F_F7B5 => "sysv4".into(),
        0x0102_1994 => "tmpfs".into(),
        0x7472_6163 => "tracefs".into(),
        0x2405_1905 => "ubifs".into(),
        0x1501_3346 => "udf".into(),
        0x0001_1954 => "ufs".into(),
        0x5419_0100 => "ufs".into(),
        0x9FA2 => "usbdevfs".into(),
        0x0102_1997 => "v9fs".into(),
        0xBACB_ACBC => "vmhgfs".into(),
        0xA501_FCF5 => "vxfs".into(),
        0x565A_4653 => "vzfs".into(),
        0x5346_4846 => "wslfs".into(),
        0xABBA_1974 => "xenfs".into(),
        0x012F_F7B4 => "xenix".into(),
        0x5846_5342 => "xfs".into(),
        0x012F_D16D => "xia".into(),
        0x2FC1_2FC1 => "zfs".into(),
        other => format!("UNKNOWN ({:#x})", other).into(),
    }
}
