use crate::vfs::Node;
use anyhow::bail;
use std::collections::HashMap;
use std::ffi::OsString;

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

    pub fn add_child(&mut self, path: OsString, child: Node) -> anyhow::Result<()> {
        if self.children.contains_key(&path) {
            bail!("Directory already contains child `{path:?}`");
        } else {
            self.children.insert(path, child);
            Ok(())
        }
    }
}
