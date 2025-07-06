use crate::vfs::Node;
use anyhow::bail;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use crate::vfs::file::File;

#[derive(Debug)]
pub struct Directory {
    children: HashMap<OsString, Node>,
}

impl Directory {
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
        }
    }

    pub fn get_or_create_subdir(&mut self, path: OsString) -> &mut Directory {
        self.children
            .entry(path) // fixme: allocation even on no insert
            .or_insert(Node::Directory(Directory {
                children: HashMap::new(),
            }))
            .as_directory_mut()
    }

    pub fn get_subdir(&self, path: &OsStr) -> Option<&Directory> {
        match self.children.get(path) {
            Some(Node::Directory(dir)) => Some(dir),
            _ => None,
        }
    }

    pub fn get_child(&self, path: &OsStr) -> Option<&Arc<File>> {
        match self.children.get(path) {
            Some(Node::File(file)) => Some(file),
            _ => None,
        }
    }

    pub fn add_child(&mut self, path: OsString, child: Node) -> anyhow::Result<()> {
        if self.children.contains_key(&path) {
            bail!("Directory already contains child `{path:?}`");
        } else {
            self.children.insert(path, child);
            Ok(())
        }
    }
}
