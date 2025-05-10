use std::borrow::Borrow;
use std::mem::MaybeUninit;
use std::path::Path;
use std::slice;
use std::sync::{Arc, Condvar, Mutex};
use anyhow::Context;
use core_protocols::{create, protocol, protocol::core::server::Sync as _};
use directory::Directory;
use file::File;

mod directory;
mod file;
mod tar;

pub fn vfs_main(started_flag: Arc<(Mutex<bool>, Condvar)>) -> anyhow::Result<()> {
	println!("starting file server...");

	#[cfg(not(target_os = "popcorn"))]
	{
		let tar_file = std::fs::read("res/test/data.tar")?;
		let vfs = Vfs::from_tar(&tar_file)?;
	}

	println!("initialised vfs from ramdisk");

	let mut server = create!(
        ":core.fs.fileserver@v1",
        impl protocol::core::server::Sync
    ).with_context(|| "failed to start file server")?;

	{
		// notify `init` that the VFS server is running and ready to handle requests
		let (lock, cvar) = &*started_flag;
		let mut started = lock.lock().unwrap();
		*started = true;
		cvar.notify_one();
	}
	drop(started_flag);

	let mut buffer = MaybeUninit::uninit();
	loop {
		// TODO: clean up this api
		let packet = server.get(slice::from_mut(&mut buffer)).unwrap();
		assert!(packet > 0, "should not return without error if zero packets received");

		// SAFETY: kernel initialised the packet buffer
		let buffer = unsafe { buffer.assume_init_mut() };
	}
}

#[derive(Debug)]
struct Vfs {
	root_node: Directory,
}

impl Vfs {
	pub fn from_tar(tar: &[u8]) -> anyhow::Result<Vfs> {
		let mut vfs = Vfs {
			root_node: Directory::new(),
		};
		let tar = tar::Tar::new(tar);
		for file in tar.files() {
			let file = file?;
			vfs.add_full_cache(file.path.borrow(), file.data)?;
		}
		Ok(vfs)
	}

	fn add_full_cache(&mut self, path: &Path, data: &[u8]) -> anyhow::Result<()> {
		let mut dir = &mut self.root_node;
		for part in path.parent().unwrap_or(Path::new("")).iter() {
			dir = dir.get_or_create_subdir(part.to_owned());
		}
		dir.add_child(
			path.file_name()
			    .ok_or(anyhow::Error::msg("file contains no filename"))?
					.to_owned(),
			Node::File(File::new_from_cache(data)),
		)
	}
}

#[derive(Debug)]
enum Node {
	File(File),
	Directory(Directory),
}

impl Node {
	#[track_caller]
	fn as_directory_mut(&mut self) -> &mut Directory {
		match self {
			Self::Directory(d) => d,
			_ => panic!("Node is not a directory"),
		}
	}
}
