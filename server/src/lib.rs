#![feature(popcorn_protocol)]
#![feature(try_blocks)]
#![feature(slice_ptr_get)]
#![feature(macro_metavar_expr_concat)]
#![feature(macro_metavar_expr)]
#![feature(asm_goto_with_outputs)]
#![feature(popcorn_std)]

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::mem::ManuallyDrop;
use std::os::popcorn::io::{AsRawHandle, OwnedHandle, AsHandle, FromRawHandle, RawHandle};
use std::os::popcorn::proto::{protocol, Protocol, ProtocolList};
use std::path::Path;
use std::ptr::{slice_from_raw_parts, with_exposed_provenance};
use executor::io::popcorn::AsyncOwnedHandle;
use std::sync::Arc;

protocol! {
	pub unsafe protocol (Sync, AsyncSync) = 8 {
		fn next@1(&self) -> std::io::Result<Packet> {
            args => [{todo!(); 1usize}];
            _ => todo!();
			/*let mut buf = MaybeUninit::<Packet>::uninit();

			unsafe {
				syscall!(1u128<<96 | UID, self.as_raw_handle().0, buf.as_mut_ptr() =>
					Ok(_res) => {
						return Ok(buf.assume_init())
					}
					Err(e) => {
						return Err(e);
					}
				);
			}*/
		}

		fn reply@2(&self, response: Response) -> std::io::Result<()> {
            args => [&response as *const Response as usize];
            _ => ();
		}

		fn forge@3<U: ProtocolList>(&self, handle_id: isize) -> std::io::Result<OwnedHandle<U>> {
            args => [
                handle_id as usize,
                U::UIDs.as_ptr() as usize,
                U::UIDs.len() * size_of::<u128>()
            ];
			
            ret => unsafe { OwnedHandle::from_raw_handle(RawHandle(ret as isize)) };
		}
	}
}

pub struct Server<T: ServerHandler> {
    handler: T,
}

impl<T: ServerHandler + 'static> Server<T> {
    pub fn new(name: impl AsRef<OsStr>, handler: impl FnOnce(AsyncOwnedHandle<&dyn AsyncSync>) -> T) -> io::Result<Self> {
        /*let handle = OwnedHandle::<&dyn Sync>::new(name)?;
        let handle = AsyncOwnedHandle::from_sync(handle);
        Ok(Self {
            handler: handler(handle),
        })*/
        todo!()
    }

    pub async fn next_packet(&self) -> io::Result<Packet> {
        AsyncSync::next(self.handle()).await
    }

    pub async fn process(&self, packet: Packet) {
        println!("received packet: {packet:#x?}");

        let result: core::result::Result<Result, Error> = if packet.uid == 1u128 << 96 {
            try {
                // open@abi.v1
                let endpoint = unsafe { str::from_utf8_unchecked(&*slice_from_raw_parts(with_exposed_provenance(packet.arg0), packet.arg1)) };
                let uids = unsafe { &*slice_from_raw_parts(with_exposed_provenance(packet.arg2), packet.arg3) };
                //let mut args = with_exposed_provenance(packet.arg4);

                let mut ctx = T::CtorContext::default();
                //let table = self.handler.dispatch_table();

                println!("ctor for {uids:#x?}");

                for &uid in uids {
                    //let deserialize = table.ctor_deserialize(uid)?;
                    let ctor = Box::<[u8]>::from([]); // fixme //deserialize(&mut args)?;
	                if let Some(f) = ctx.visitors().table.get(&uid) {
		                f(&mut ctx, ctor.as_ptr())?;
	                } else if let Some(f) = ctx.visitors().default {
		                f(&mut ctx, uid)?;
	                } else {
		                Err(Error::UnsupportedProtocol)?;
	                }
                }
                Result::from(self.handler.ctor(Path::new(endpoint), ctx).await?)
            }
        } else {
            let table = self.handler.dispatch_table();
            table.dispatch(
                packet.uid,
                &self.handler as *const _ as *const _,
                packet.arg0,
                packet.arg1,
                packet.arg2,
                packet.arg3,
                packet.arg4,
            ).await
        };

        let result_clone = result.as_ref().map(Result::clone_private).map_err(|e| *e);
        let error = result.is_err();

        Sync::reply(&self.handler.handle().as_handle(), Response {
            result: result.unwrap_or_else(|e| Result::Value(e as u128)),
            packet: packet.packet,
            error,
        }).expect("server reply error");

        if let Ok(Result::SelfHandle(_, ptr, len)) = result_clone {
            let _ = unsafe { Box::from_raw(core::ptr::slice_from_raw_parts_mut(ptr.cast_mut(), len)) };
        }
    }
    
    pub fn run(self: Arc<Self>) -> ! {
        executor::block_on(async move {
            loop {
                let packet = self.next_packet().await.expect("failed to receive packet");

                let this = self.clone();
                executor::spawn(async move {
                    this.process(packet).await
                });
            }
        })
    }

    pub fn handle(&self) -> &AsyncOwnedHandle<&'static dyn AsyncSync> {
        self.handler.handle()
    }

	pub fn inner(&self) -> &T {
		&self.handler
	}

	pub fn inner_mut(&mut self) -> &mut T {
		&mut self.handler
	}
}

pub trait ServerHandler {
    type CtorContext: CtorContext;

    async fn ctor(&self, endpoint: &Path, ctx: Self::CtorContext) -> core::result::Result<ReturnHandle, Error>;
    async fn destroy(&self, handle: isize) -> core::result::Result<(), Error>;
    fn dispatch_table(&self) -> &'static DispatchTable;

    fn handle(&self) -> &AsyncOwnedHandle<&'static dyn AsyncSync>;
}

pub trait CtorContext where Self: 'static + Default {
    fn visitors(&self) -> &'static ProtocolVisitor<Self>;
}

pub struct ProtocolVisitor<U: ?Sized> {
    table: HashMap<u128, fn(&mut U, *const u8) -> core::result::Result<(), Error>>,
	default: Option<fn(&mut U, u128) -> core::result::Result<(), Error>>,
}

impl<U: ?Sized> ProtocolVisitor<U> {
    pub fn new() -> Self {
	    Self {
		    table: HashMap::new(),
		    default: None,
	    }
    }

    pub fn add_visitor<T: Protocol + ?Sized>(mut self, f: fn(&mut U, &T::Ctor) -> core::result::Result<(), Error>) -> Self {
        self.table.insert(T::UID, unsafe { core::mem::transmute(f) });
        self
    }

	pub fn add_default(mut self, f: fn(&mut U, u128) -> core::result::Result<(), Error>) -> Self {
		self.default = Some(f);
		self
	}
}

#[derive(Debug)]
pub struct DispatchTable {
    map: HashMap<u128, for<'a> fn(&'a (), usize, usize, usize, usize, usize) -> Pin<Box<dyn 'a + Send + Future<Output = core::result::Result<Result, Error>>>>>,
}

impl DispatchTable {
    pub fn new() -> Self { Self { map: HashMap::new() } }
    pub fn add_vtable(mut self, vtable: HashMap<u128, for<'a> fn(&'a (), usize, usize, usize, usize, usize) -> Pin<Box<dyn 'a + Send + Future<Output = core::result::Result<Result, Error>>>>>) -> Self {
        self.map.extend(vtable);
        self
    }

    async fn dispatch(
        &self,
        uid: u128,
        f_self: *const (),
        arg0: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
        arg4: usize
    ) -> core::result::Result<Result, Error> {
        let f = self.map.get(&uid).ok_or(Error::UnsupportedProtocol)?;
        f(unsafe { &*f_self }, arg0, arg1, arg2, arg3, arg4).await
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Packet {
    pub uid: u128,
    pub packet: usize,
    pub arg0: usize,
    pub arg1: usize,
    pub arg2: usize,
    pub arg3: usize,
    pub arg4: usize,
}

#[derive(Debug)]
#[repr(C)]
pub struct Response {
    pub result: Result,
    pub packet: usize,
    pub error: bool,
}

#[derive(Debug)]
#[repr(C)]
pub enum Result {
    SelfHandle(isize, *const u128, usize),
    SelfDefaultHandle(isize),
    TransferHandle(isize),
    Value(u128),
}

impl Result {
    fn clone_private(&self) -> Result {
        match self {
            Self::SelfHandle(id, ptr, len) => Self::SelfHandle(*id, *ptr, *len),
            Self::SelfDefaultHandle(id) => Self::SelfDefaultHandle(*id),
            Self::TransferHandle(id) => Self::TransferHandle(*id),
            Self::Value(val) => Self::Value(*val),
        }
    }
}

pub enum ReturnHandle {
    /// Transfers ownership of a handle to the caller, with support for all the protocols that the original handle had
    Transfer(OwnedHandle),
    /// Creates a handle to a new object, with support for the protocols passed in
    New(isize, Box<[u128]>),
    /// Creates a handle to a new object, with support for all the protocols that were requested by the caller
    ///
    /// Only valid to return from a constructor function.
    /// Panics otherwise.
    NewDefault(isize),
}

impl From<ReturnHandle> for Result {
    fn from(value: ReturnHandle) -> Result {
        match value {
            ReturnHandle::Transfer(owned) => {
                let owned = ManuallyDrop::new(owned);
                Result::TransferHandle(owned.as_raw_handle().0)
            },
            ReturnHandle::New(id, protos) => {
                let protos = Box::into_raw(protos);
                Result::SelfHandle(id, protos.cast_const().as_ptr(), protos.len())
            },
            ReturnHandle::NewDefault(id) => {
                Result::SelfDefaultHandle(id)
            },
        }
    }
}
