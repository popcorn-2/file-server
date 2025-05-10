use anyhow::Context;
use std::borrow::Cow;
use std::ffi::CStr;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use zerocopy::{FromBytes, Immutable, KnownLayout};

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C, packed)]
struct Header {
    file_name: [u8; 100],
    file_mode: OctalStr<8>,
    uid: OctalStr<8>,
    gid: OctalStr<8>,
    size: OctalStr<12>,
    mtime: OctalStr<12>,
    checksum: u64,
    ty: u8,
    link_name: [u8; 100],
    ustar: [u8; 6],
    _pad: [u8; 249],
}

const _: () = {
    assert!(size_of::<Header>() == 512);
};

impl Debug for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let checksum = self.checksum;
        f.debug_struct("Header")
            .field("file_name", &self.file_name)
            .field("file_mode", &self.file_mode)
            .field("uid", &self.uid)
            .field("gid", &self.gid)
            .field("size", &self.size)
            .field("mtime", &self.mtime)
            .field("checksum", &checksum)
            .field("ty", &self.ty)
            .field("ustar", &self.ustar)
            .finish_non_exhaustive()
    }
}

pub struct Tar<'a> {
    data: &'a [u8],
}

impl<'a> Tar<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn files(&self) -> Iter<'a> {
        Iter { data: self.data }
    }
}

pub struct Iter<'a> {
    data: &'a [u8],
}

impl<'a> Iter<'a> {
    fn next_inner(&mut self) -> anyhow::Result<Option<File<'a>>> {
        let (header, data) = Header::ref_from_prefix(self.data)
            .map_err(|e| e.map_src(|_| &()))
            .with_context(|| "failed to parse tar file header")?;

        if header.ustar != *b"ustar\0" {
            return Ok(None);
        }

        let (file_data, data) = data.split_at(header.size.as_usize());

        let align_point = if header.size.as_usize() & 511 == 0 {
            0
        } else {
            (header.size.as_usize() | 511) + 1
        };

        self.data = data.split_at(align_point - header.size.as_usize()).1;

        let path = CStr::from_bytes_until_nul(&header.file_name)
            .with_context(|| "failed to parse filename as C string")?;
        let utf8 = String::from_utf8_lossy(path.to_bytes());
        let path = match utf8 {
            Cow::Borrowed(b) => Cow::Borrowed(Path::new(b)),
            Cow::Owned(o) => Cow::Owned(PathBuf::from(o)),
        };

        Ok(Some(File {
            path,
            data: file_data,
        }))
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = anyhow::Result<File<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_inner().transpose()
    }
}

#[derive(Debug, PartialEq)]
pub struct File<'a> {
    pub path: Cow<'a, Path>,
    pub data: &'a [u8],
}

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(transparent)]
struct OctalStr<const N: usize>([u8; N]);

impl<const N: usize> Debug for OctalStr<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for c in self.0.iter().take(N - 1) {
            write!(f, "{}", *c as char)?;
        }
        write!(f, "\"")
    }
}

impl<const N: usize> OctalStr<N> {
    pub fn as_usize(&self) -> usize {
        let mut ret = 0;

        for c in self.0.iter().take(N - 1) {
            ret *= 8;
            ret += usize::from(c - b'0');
        }

        ret
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_files() {
        let data = include_bytes!("../res/test/basic.tar");

        let files = Tar::new(data);
        let mut files = files.files();

        assert_eq!(
            files.next().unwrap().unwrap(),
            File {
                path: Cow::Borrowed(Path::new("foo.txt")),
                data: &[],
            }
        );
        assert_eq!(
            files.next().unwrap().unwrap(),
            File {
                path: Cow::Borrowed(Path::new("foo/bar.txt")),
                data: &[],
            }
        );
        assert!(files.next().is_none());
    }

    #[test]
    fn read_data() {
        let data = include_bytes!("../res/test/data.tar");

        let files = Tar::new(data);
        let mut files = files.files();

        assert_eq!(
            files.next().unwrap().unwrap(),
            File {
                path: Cow::Borrowed(Path::new("foo.txt")),
                data: b"hello\n",
            }
        );
        assert_eq!(
            files.next().unwrap().unwrap(),
            File {
                path: Cow::Borrowed(Path::new("foo/bar.txt")),
                data: b"world\n",
            }
        );
        assert!(files.next().is_none());
    }
}
