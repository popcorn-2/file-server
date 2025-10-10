#![feature(popcorn_protocol)]
#![feature(try_blocks)]
#![feature(slice_ptr_get)]
#![feature(macro_metavar_expr_concat)]
#![feature(asm_goto_with_outputs)]

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::mem::ManuallyDrop;
use std::os::popcorn::handle::{AsRawHandle, OwnedHandle, AsHandle};
use std::os::popcorn::proto::{Error, Protocol};
use std::path::Path;
use std::pin::{Pin, pin};
use std::ptr::{slice_from_raw_parts, with_exposed_provenance};
use executor::io::popcorn::AsyncOwnedHandle;
use std::os::popcorn::proto::ProtocolTuple;
use std::os::popcorn::handle::FromRawHandle;
use std::os::popcorn::handle::RawHandle;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

macro_rules! syscall_async {
    ($uid:expr $(, $arg:expr)* => Ok($res_h:ident) => $happy:block Err($res_e:ident) => $error:block) => {
        let val = executor::async_syscall!($uid $(, $arg)*).await;
        match val {
            Ok($res_h) => $happy,
            Err($res_e) => $error,
        }
    }
}

macro_rules! syscall_sync {
    ($uid:expr => Ok($res_h:ident) => $happy:block Err($res_e:ident) => $error:block) => {
        let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "clc",
            "syscall",
            "jc {error}",
            inout("rax") $uid as u64 => low,
            out("rcx") _,
            out("rdx") high,
            out("rdi") _,
            out("rsi") _,
            out("r8") _,
            inout("r9") (($uid as u128) >> 64) as u64 => _,
            out("r10") _,
            out("r11") _,
            out("r12") _,
            error = label { match ::std::io::Error::from_raw_os_error(low as isize) { $res_e => $error } }
        );
        match (high as u128) << 64 | (low as u128) { $res_h => $happy }
    };
    ($uid:expr, $arg0:expr => Ok($res_h:ident) => $happy:block Err($res_e:ident) => $error:block) => {
        let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "clc",
            "syscall",
            "jc {error}",
            inout("rax") $uid as u64 => low,
            out("rcx") _,
            out("rdx") high,
            out("rsi") _,
            inout("rdi") $arg0 as usize => _,
            out("r8") _,
            inout("r9") (($uid as u128) >> 64) as u64 => _,
            out("r10") _,
            out("r11") _,
            out("r12") _,
            error = label { match ::std::io::Error::from_raw_os_error(low as isize) { $res_e => $error } }
        );
        match (high as u128) << 64 | (low as u128) { $res_h => $happy }
    };
    ($uid:expr, $arg0:expr, $arg1:expr => Ok($res_h:ident) => $happy:block Err($res_e:ident) => $error:block) => {
        let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "clc",
            "syscall",
            "jc {error}",
            inout("rax") $uid as u64 => low,
            out("rcx") _,
            out("rdx") high,
            inout("rdi") $arg0 as usize => _,
            inout("rsi") $arg1 as usize => _,
            out("r8") _,
            inout("r9") (($uid as u128) >> 64) as u64 => _,
            out("r10") _,
            out("r11") _,
            out("r12") _,
            error = label { match ::std::io::Error::from_raw_os_error(low as isize) { $res_e => $error } }
        );
        match (high as u128) << 64 | (low as u128) { $res_h => $happy }
    };
    ($uid:expr, $arg0:expr, $arg1:expr, $arg2:expr => Ok($res_h:ident) => $happy:block Err($res_e:ident) => $error:block) => {
        let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "clc",
            "syscall",
            "jc {error}",
            inout("rax") $uid as u64 => low,
            out("rcx") _,
            inout("rdx") $arg2 as usize => high,
            inout("rdi") $arg0 as usize => _,
            inout("rsi") $arg1 as usize => _,
            out("r8") _,
            inout("r9") (($uid as u128) >> 64) as u64 => _,
            out("r10") _,
            out("r11") _,
            out("r12") _,
            error = label { match ::std::io::Error::from_raw_os_error(low as isize) { $res_e => $error } }
        );
        match (high as u128) << 64 | (low as u128) { $res_h => $happy }
    };
    ($uid:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3: expr => Ok($res_h:ident) => $happy:block Err($res_e:ident) => $error:block) => {
        let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "clc",
            "syscall",
            "jc {error}",
            inout("rax") $uid as u64 => low,
            out("rcx") _,
            inout("rdx") $arg2 as usize => high,
            inout("rdi") $arg0 as usize => _,
            inout("rsi") $arg1 as usize => _,
            out("r8") _,
            inout("r9") (($uid as u128) >> 64) as u64 => _,
            inout("r10") $arg3 as usize => _,
            out("r11") _,
            out("r12") _,
            error = label { match ::std::io::Error::from_raw_os_error(low as isize) { $res_e => $error } }
        );
        match (high as u128) << 64 | (low as u128) { $res_h => $happy }
    };
    ($uid:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr => Ok($res_h:ident) => $happy:block Err($res_e:ident) => $error:block) => {
        let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "clc",
            "syscall",
            "jc {error}",
            inout("rax") $uid as u64 => low,
            out("rcx") _,
            inout("rdx") $arg2 as usize => high,
            inout("rdi") $arg0 as usize => _,
            inout("rsi") $arg1 as usize => _,
            inout("r8") $arg4 as usize => _,
            inout("r9") (($uid as u128) >> 64) as u64 => _,
            inout("r10") $arg3 as usize => _,
            out("r11") _,
            out("r12") _,
            error = label { match ::std::io::Error::from_raw_os_error(low as isize) { $res_e => $error } }
        );
        match (high as u128) << 64 | (low as u128) { $res_h => $happy }
    };
}

macro_rules! protocol {
    () => {};
    (pub protocol $name:ident = $uid:literal {
        ctor => {
            $($ctor_arg:ident : $ctor_ty:ty),* $(,)?
        }

        $(fn ~$fn_name_d:ident($this_d:ident $(, $fn_arg_d:ident: $fn_ty_d:ty)* $(,)?) $(-> $fn_ret_d:ty)? $f_d:block)*
        $(fn $fn_name:ident $(<$($gen_ident:ident $(: $gen_bound:path)?),* $(,)?>)? (&$this:ident $(, $fn_arg:ident: $fn_ty:ty)* $(,)?) $(-> $fn_ret:ty)? $f:block)*
    } $($rest:tt)*) => {
        #[repr(C)]
        pub struct $name {
            $(pub $ctor_arg : $ctor_ty),*
        }

        impl std::os::popcorn::proto::Protocol for $name {
            type Ctor = Self;
            const UID: u128 = $uid;
        }

        pub trait ${concat($name, Tr)} {
            $(fn $fn_name_d($this_d $(, $fn_arg_d: $fn_ty_d)*) $(-> std::io::Result<$fn_ret_d>)? where Self: Sized;)*
            $(fn $fn_name $(<$($gen_ident $(: $gen_bound)?),*>)? (&$this $(, $fn_arg: $fn_ty)*) $(-> std::io::Result<$fn_ret>)? where Self: Sized;)*
        }

        pub trait ${concat(Async, $name, Tr)} {
            $(async fn $fn_name_d($this_d $(, $fn_arg_d: $fn_ty_d)*) $(-> std::io::Result<$fn_ret_d>)? where Self: Sized;)*
            $(async fn $fn_name $(<$($gen_ident $(: $gen_bound)?),*>)? (&$this $(, $fn_arg: $fn_ty)*) $(-> std::io::Result<$fn_ret>)? where Self: Sized;)*
        }

        impl<T: std::os::popcorn::proto::HasProtocol<$name>> ${concat($name, Tr)} for std::os::popcorn::handle::OwnedHandle<T> {
            $(fn $fn_name_d($this_d $(, $fn_arg_d: $fn_ty_d)*) $(-> std::io::Result<$fn_ret_d>)? where Self: Sized { use syscall_sync as syscall; #[allow(dead_code)] const UID: u128 = $uid; $f_d })*
            $(fn $fn_name $(<$($gen_ident $(: $gen_bound)?),*>)? (&$this $(, $fn_arg: $fn_ty)*) $(-> std::io::Result<$fn_ret>)? where Self: Sized { use syscall_sync as syscall; #[allow(dead_code)] const UID: u128 = $uid; $f })*
        }
        impl<T: std::os::popcorn::proto::HasProtocol<$name>> ${concat($name, Tr)} for std::os::popcorn::handle::BorrowedHandle<'_, T> {
            $(fn $fn_name_d($this_d $(, $fn_arg_d: $fn_ty_d)*) $(-> std::io::Result<$fn_ret_d>)? where Self: Sized { use syscall_sync as syscall; #[allow(dead_code)] const UID: u128 = $uid; $f_d })*
            $(fn $fn_name $(<$($gen_ident $(: $gen_bound)?),*>)? (&$this $(, $fn_arg: $fn_ty)*) $(-> std::io::Result<$fn_ret>)? where Self: Sized { use syscall_sync as syscall; #[allow(dead_code)] const UID: u128 = $uid; $f })*
        }
        impl<T: std::os::popcorn::proto::HasProtocol<$name>> ${concat(Async, $name, Tr)} for executor::io::popcorn::AsyncOwnedHandle<T> {
            $(async fn $fn_name_d($this_d $(, $fn_arg_d: $fn_ty_d)*) $(-> std::io::Result<$fn_ret_d>)? where Self: Sized { use syscall_async as syscall; #[allow(dead_code)] const UID: u128 = $uid; $f_d })*
            $(async fn $fn_name $(<$($gen_ident $(: $gen_bound)?),*>)? (&$this $(, $fn_arg: $fn_ty)*) $(-> std::io::Result<$fn_ret>)? where Self: Sized { use syscall_async as syscall; #[allow(dead_code)] const UID: u128 = $uid; $f })*
        }

        protocol!($($rest)*);
    };
}

protocol! {
	pub protocol Sync = 8 {
		ctor => {}

		fn next(&self) -> Packet {
			let mut buf = MaybeUninit::<Packet>::uninit();

			unsafe {
				syscall!(1u128<<96 | UID, self.as_raw_handle().0, buf.as_mut_ptr() =>
					Ok(_res) => {
						return Ok(buf.assume_init())
					}
					Err(e) => {
						return Err(e);
					}
				);
			}
		}

		fn reply(&self, response: Response) -> () {
			unsafe {
				syscall!(2u128<<96 | UID, self.as_raw_handle().0, &response as *const Response =>
					Ok(_res) => {
						return Ok(())
					}
					Err(e) => {
						return Err(e);
					}
				);
			}
		}

		fn forge<U: ProtocolTuple>(&self, handle: isize) -> OwnedHandle<U> {
			unsafe {
				syscall!(3u128<<96 | UID, self.as_raw_handle().0, handle, U::UID.as_ptr(), U::UID.len() * size_of::<u128>() =>
					Ok(res) => {
						return Ok(unsafe { OwnedHandle::from_raw_handle(RawHandle(res as isize)) });
					}
					Err(e) => {
						return Err(e);
					}
				);
			}
		}
	}
}

pub struct Server<T: ServerHandler> {
    handler: T,
}

impl<T: ServerHandler + 'static> Server<T> {
    pub fn new(name: impl AsRef<OsStr>, handler: impl FnOnce(AsyncOwnedHandle<Sync>) -> T) -> io::Result<Self> {
        let handle = OwnedHandle::<Sync>::new(name, Sync {})?;
        let handle = AsyncOwnedHandle::from_sync(handle);
        Ok(Self {
            handler: handler(handle),
        })
    }

    pub async fn next_packet(&self) -> io::Result<Packet> {
        AsyncSyncTr::next(self.handler.handle()).await
    }

    async fn process(&self, packet: Packet) {
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
                    let f = ctx.visitors().table.get(&uid).ok_or(Error::UnsupportedProtocol)?;
                    f(&mut ctx, ctor.as_ptr())?;
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

        SyncTr::reply(&self.handler.handle().as_handle(), Response {
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

    pub fn handle(&self) -> &AsyncOwnedHandle<Sync> {
        self.handler.handle()
    }
}

pub trait ServerHandler {
    type CtorContext: CtorContext;

    async fn ctor(&self, endpoint: &Path, ctx: Self::CtorContext) -> core::result::Result<ReturnHandle, Error>;
    async fn destroy(&self, handle: isize) -> core::result::Result<(), Error>;
    fn dispatch_table(&self) -> &'static DispatchTable;

    fn handle(&self) -> &AsyncOwnedHandle<Sync>;
}

pub trait CtorContext where Self: 'static + Default {
    fn visitors(&self) -> &'static ProtocolVisitor<Self>;
}

pub struct ProtocolVisitor<U: ?Sized> {
    table: HashMap<u128, fn(&mut U, *const u8) -> core::result::Result<(), Error>>,
}

impl<U: ?Sized> ProtocolVisitor<U> {
    pub fn new() -> Self { Self { table: HashMap::new() } }

    pub fn add_visitor<T: Protocol + ?Sized>(mut self, f: fn(&mut U, &T::Ctor) -> core::result::Result<(), Error>) -> Self {
        self.table.insert(T::UID, unsafe { core::mem::transmute(f) });
        self
    }
}

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
