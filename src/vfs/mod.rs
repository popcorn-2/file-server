use std::borrow::Borrow;
use std::bstr::ByteStr;
use std::cell::RefCell;
use std::cmp::min;
use std::ffi::{OsStr, OsString};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::os::popcorn::handle::{AsHandle, AsRawHandle, BorrowedHandle, OwnedHandle};
use std::path::Path;
use std::slice;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::Context;
use directory::Directory;
use file::File;
use std::os::popcorn::proto::server::{self, ProtocolVisitor, CtorContext, DispatchTable, ServerHandler, ReturnHandle, SyncTr as _};
use std::os::popcorn::proto::Error;
use std::os::popcorn::proto::{Protocol, proc::BuilderTr as _};
use slab::Slab;

mod directory;
mod file;
mod tar;
mod proto;

pub fn vfs_main(started_flag: Arc<AtomicBool>, data_pointer: &[u8]) -> anyhow::Result<()> {
	println!("starting file server...");

	let vfs = Vfs::from_tar(data_pointer)?;

	println!("initialised vfs from ramdisk");
	println!("{vfs:#?}");

	let srv = server::Server::new(":fs", |handle| ServerState::new(vfs, handle))
			.context("failed to start fs server")?;

	started_flag.store(true, Ordering::SeqCst);
	drop(started_flag);

	srv.event_loop()
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
			println!("found file `{}`", file.path.display());
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

	fn get_file(&self, path: &Path) -> Option<&Arc<File>> {
		let mut dir = &self.root_node;
		for part in path.parent().unwrap_or(Path::new("")).iter() {
			dir = dir.get_subdir(part)?;
		}
		dir.get_child(path.file_name()?)
	}
}

#[derive(Debug)]
enum Node {
	File(Arc<File>),
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

struct ServerState {
	vfs: Vfs,
	open_files: RefCell<Slab<HandleState>>,
	handle: OwnedHandle<server::Sync>,
}

struct HandleState {
	offset: usize,
	file: Arc<File>,
}

impl ServerState {
	fn new(vfs: Vfs, handle: OwnedHandle<server::Sync>) -> Self {
		Self {
			vfs,
			open_files: RefCell::new(Slab::new()),
			handle,
		}
	}

	fn open_file(&self, path: &Path) -> Result<isize, Error> {
		let state = self.vfs.get_file(path).ok_or(Error::EndpointNotFound)?;
		let handle = self.open_files.borrow_mut().insert(HandleState {
			offset: 0,
			file: Arc::clone(state),
		});
		println!("open {} as {handle}", path.display());
		Ok(handle as isize)
	}
}

impl ServerHandler for ServerState {
	type CtorContext = CtorCtx;

	fn ctor(&self, endpoint: &Path, ctx: Self::CtorContext) -> Result<ReturnHandle, Error> {
		println!("open {} with {ctx:?}", endpoint.display());
		match ctx {
			CtorCtx::Unknown => return Err(Error::UnsupportedProtocol),
			CtorCtx::File { write: true } => { return Err(Error::UnsupportedProtocol); } // read-only filesystem for now
			CtorCtx::File { .. } => {
				let handle = self.open_file(endpoint)?;
				Ok(ReturnHandle::NewDefault(handle))
			},
			CtorCtx::Process => {
				let file_handle = self.open_file(endpoint)?;
				let file_handle = self.handle.forge::<(std::os::popcorn::proto::io::Read, std::os::popcorn::proto::io::Seek)>(file_handle)?;
				
				let program_name = endpoint.file_stem().unwrap_or_else(|| endpoint.as_os_str());
				let mut elf_path = OsString::from("elf:");
				elf_path.push(program_name);
				let handle = OwnedHandle::<std::os::popcorn::proto::proc::Builder>::new_from(elf_path, file_handle)?;

				println!("vfs builder handle: {handle:?}");

				Ok(ReturnHandle::Transfer(handle.type_erase()))
			}
		}
	}

	fn destroy(&self, handle: isize) -> Result<(), Error> {
		self.open_files.borrow_mut().remove(handle as usize);
		Ok(())
	}

	fn dispatch_table(&self) -> &'static DispatchTable {
		static DISPATCH: OnceLock<DispatchTable> = OnceLock::new();

		DISPATCH.get_or_init(|| DispatchTable::new()
				.add_vtable(<Self as proto::CoreIoRead>::__vtable())
				.add_vtable(<Self as proto::CoreIoWrite>::__vtable())
				.add_vtable(<Self as proto::CoreIoSeek>::__vtable())
				.add_vtable(<Self as proto::CoreFsFile>::__vtable())
				.add_vtable(<Self as proto::CoreProcBuilder>::__vtable())
		)
	}

	fn handle(&self) -> BorrowedHandle<'_, server::Sync> { self.handle.as_handle() }
}

impl proto::CoreIoRead for ServerState {
	fn new_from(&self, endpoint: &Path, handle: OwnedHandle) -> Result<ReturnHandle, Error> {
		Err(Error::UnsupportedProtocol)
	}

	fn read(&self, handle: isize, output_size: usize) -> Result<Box<[u8]>, Error> {
		println!("read {output_size} bytes from {handle}");
		let mut guard = self.open_files.borrow_mut();
		let state = guard.get_mut(handle as usize)
				.ok_or(Error::InvalidHandle)?;
		let res = state.file.read(state.offset..(state.offset + output_size));
		state.offset += res.len();
		println!("read {} bytes ({})", res.len(), ByteStr::new(&res));
		Ok(res)
	}
}

impl proto::CoreIoWrite for ServerState {
	fn new_from(&self, endpoint: &Path, handle: OwnedHandle) -> Result<ReturnHandle, Error> {
		Err(Error::UnsupportedProtocol)
	}

	fn write(&self, handle: isize, buf: &[u8]) -> Result<usize, Error> {
		let buf = std::bstr::ByteStr::new(buf);
		println!("write {buf} to {handle}");
		Ok(0)
	}
}

impl proto::CoreFsFile for ServerState {
	fn new_from(&self, endpoint: &Path, handle: OwnedHandle) -> Result<ReturnHandle, Error> {
		Err(Error::UnsupportedProtocol)
	}
}

impl proto::CoreIoSeek for ServerState {
	fn new_from(&self, endpoint: &Path, handle: OwnedHandle) -> Result<ReturnHandle, Error> {
		Err(Error::UnsupportedProtocol)
	}

	fn tell(&self, handle: isize) -> Result<usize, Error> {
		let mut guard = self.open_files.borrow();
		let state = guard.get(handle as usize)
		                 .ok_or(Error::InvalidHandle)?;
		Ok(state.offset)
	}

	fn set_pos(&self, handle: isize, pos: usize) -> Result<(), Error> {
		let mut guard = self.open_files.borrow_mut();
		let state = guard.get_mut(handle as usize)
		                 .ok_or(Error::InvalidHandle)?;
		let pos = min(pos, state.file.meta_size);
		state.offset = pos;
		Ok(())
	}
}

impl proto::CoreProcBuilder for ServerState {
	fn spawn(&self, handle: isize) -> Result<ReturnHandle, Error> {
		Err(Error::UnsupportedProtocol)
	}

	fn add_handle(&self, handle: isize, name: &str, added_handle: OwnedHandle) -> Result<(), Error> {
		Err(Error::UnsupportedProtocol)
	}

	fn new_from(&self, endpoint: &Path, handle: OwnedHandle) -> Result<ReturnHandle, Error> {
		Err(Error::UnsupportedProtocol)
	}
}

#[derive(Default, Debug)]
enum CtorCtx {
	#[default]
	Unknown,
	File { write: bool },
	Process,
}

impl CtorCtx {
	fn set_file(&mut self) -> Result<(), Error> {
		*self = match self {
			CtorCtx::Unknown => Self::File { write: false },
			CtorCtx::File { write } => Self::File { write: *write },
			CtorCtx::Process => return Err(Error::UnsupportedProtocol),
		};
		Ok(())
	}

	fn set_write(&mut self) -> Result<(), Error> {
		*self = match self {
			CtorCtx::Unknown | CtorCtx::File { .. } => Self::File { write: true },
			CtorCtx::Process => return Err(Error::UnsupportedProtocol),
		};
		Ok(())
	}

	fn set_process(&mut self) -> Result<(), Error> {
		*self = match self {
			CtorCtx::Unknown | CtorCtx::Process => Self::Process,
			CtorCtx::File { .. } => return Err(Error::UnsupportedProtocol),
		};
		Ok(())
	}
}

impl CtorContext for CtorCtx {
	fn visitors(&self) -> &'static ProtocolVisitor<Self> {
		static VISISTORS: OnceLock<ProtocolVisitor<CtorCtx>> = OnceLock::new();

		VISISTORS.get_or_init(|| ProtocolVisitor::new()
				.add_visitor::<dyn proto::CoreIoRead>(|ctx: &mut CtorCtx, _| ctx.set_file())
				.add_visitor::<dyn proto::CoreIoWrite>(|ctx: &mut CtorCtx, _| ctx.set_write())
				.add_visitor::<dyn proto::CoreIoSeek>(|ctx: &mut CtorCtx, _| ctx.set_file())
				.add_visitor::<dyn proto::CoreFsFile>(|ctx: &mut CtorCtx, _| ctx.set_file())
				.add_visitor::<dyn proto::CoreProcBuilder>(|ctx: &mut CtorCtx, _| ctx.set_process())
		)
	}
}
