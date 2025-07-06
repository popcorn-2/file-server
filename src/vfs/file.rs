use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug)]
pub struct File {
    pub cache: FileCache,
    pub meta_size: usize,
}

impl File {
    pub fn new(size: usize) -> Arc<File> {
        Arc::new(File {
            cache: FileCache::new(),
            meta_size: size,
        })
    }

    pub fn new_from_cache(data: &[u8]) -> Arc<File> {
        Arc::new(File {
            cache: FileCache {
                pieces: vec![(0..data.len(), Box::from(data))],
            },
            meta_size: data.len(),
        })
    }

    pub fn read(&self, mut read_range: Range<usize>) -> Box<[u8]> {
        let mut data = Vec::with_capacity(min(read_range.len(), self.meta_size));
        read_range.end = min(read_range.end, self.meta_size);
        self.cache.read(read_range, &mut data);
        data.into_boxed_slice()
    }
}

struct FileCache {
    pieces: Vec<(Range<usize>, Box<[u8]>)>,
}

impl FileCache {
    pub fn new() -> Self {
        Self { pieces: vec![] }
    }
    
    pub fn read(&self, mut read_range: Range<usize>, buf: &mut Vec<u8>) {
        println!("read {read_range:?}");
        
        let (idx_start, offset_start) = self.pieces.iter().enumerate().find_map(|(idx, (range, _))| {
            if range.contains(&read_range.start) {
                Some((idx, read_range.start - range.start))
            } else { None }
        }).expect("not in cache");
        
        let (idx_end, offset_end) = self.pieces.iter().enumerate().find_map(|(idx, (range, _))| {
            if range.contains(&(read_range.end - 1)) {
                Some((idx, read_range.end - range.start))
            } else { None }
        }).expect("not in cache");
        
        if idx_start == idx_end { buf.extend(&self.pieces[idx_start].1[offset_start..offset_end]); }
        else {
            buf.extend(&self.pieces[idx_start].1[offset_start..]);
            for idx in (idx_start+1)..idx_end {
                let piece = &self.pieces[idx];
                buf.extend(&piece.1);

                if piece.0.end != self.pieces[idx + 1].0.start {
                    todo!("gap in cache")
                }
            }
            buf.extend(&self.pieces[idx_end].1[..offset_end]);
        }
    }
}

impl Debug for FileCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileCache")
                .field_with("pieces", |f| {
                    let mut d = f.debug_list();
                    for piece in &self.pieces {
                        d.entry_with(|f| {
                            f.debug_tuple("")
                                    .field(&piece.0)
                                    .finish_non_exhaustive()
                        });
                    }
                    d.finish()
                })
                .finish()
    }
}
