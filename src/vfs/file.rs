use std::ops::Range;

#[derive(Debug)]
pub struct File {
    cache: FileCache,
}

impl File {
    pub fn new(size: usize) -> File {
        File {
            cache: FileCache::new(),
        }
    }

    pub fn new_from_cache(data: &[u8]) -> File {
        File {
            cache: FileCache {
                pieces: vec![(0..data.len(), Box::from(data))],
            },
        }
    }
}

#[derive(Debug)]
struct FileCache {
    pieces: Vec<(Range<usize>, Box<[u8]>)>,
}

impl FileCache {
    pub fn new() -> Self {
        Self { pieces: vec![] }
    }
}
