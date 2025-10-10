use std::ffi::OsStr;
use std::{io, mem};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::os::popcorn::ffi::OsStrExt;
use std::os::popcorn::handle::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::os::popcorn::proto::{Protocol, ProtocolTuple};
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};
use slab::Slab;

#[macro_export]
macro_rules! async_syscall {
	($uid:expr) => {{
        let mut guard = $crate::__macro_private::SYSCALL_RESULTS.lock().unwrap();
        let key = guard.insert($crate::__macro_private::SyscallState::Pending);
        drop(guard);

	    let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "stc",
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
            in("r15") key,
            error = label { return Err(::std::io::Error::from_raw_os_error(low as isize)); }
        );

        $crate::__macro_private::Syscall::__private_new(key)
	}};

	($uid:expr, $arg0:expr) => {{
        let mut guard = $crate::__macro_private::SYSCALL_RESULTS.lock().unwrap();
        let key = guard.insert($crate::__macro_private::SyscallState::Pending);
        drop(guard);

	    let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "stc",
            "syscall",
            "jc {error}",
            inout("rax") $uid as u64 => low,
            out("rcx") _,
            out("rdx") high,
            inout("rdi") $arg0 as usize => _,
            out("rsi") _,
            out("r8") _,
            inout("r9") (($uid as u128) >> 64) as u64 => _,
            out("r10") _,
            out("r11") _,
            out("r12") _,
            in("r15") key,
            error = label { return Err(::std::io::Error::from_raw_os_error(low as isize)); }
        );

        $crate::__macro_private::Syscall::__private_new(key)
	}};

	($uid:expr, $arg0:expr, $arg1:expr) => {{
        let mut guard = $crate::__macro_private::SYSCALL_RESULTS.lock().unwrap();
        let key = guard.insert($crate::__macro_private::SyscallState::Pending);
        drop(guard);

	    let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "stc",
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
            in("r15") key,
            error = label { return Err(::std::io::Error::from_raw_os_error(low as isize)); }
        );

        $crate::__macro_private::Syscall::__private_new(key)
	}};

	($uid:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {{
        let mut guard = $crate::__macro_private::SYSCALL_RESULTS.lock().unwrap();
        let key = guard.insert($crate::__macro_private::SyscallState::Pending);
        drop(guard);

	    let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "stc",
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
            in("r15") key,
            error = label { return Err(::std::io::Error::from_raw_os_error(low as isize)); }
        );

        $crate::__macro_private::Syscall::__private_new(key)
	}};

	($uid:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {{
        let mut guard = $crate::__macro_private::SYSCALL_RESULTS.lock().unwrap();
        let key = guard.insert($crate::__macro_private::SyscallState::Pending);
        drop(guard);

	    let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "stc",
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
            in("r15") key,
            error = label { return Err(::std::io::Error::from_raw_os_error(low as isize)); }
        );

        $crate::__macro_private::Syscall::__private_new(key)
	}};

    ($uid:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {{
        let mut guard = $crate::__macro_private::SYSCALL_RESULTS.lock().unwrap();
        let key = guard.insert($crate::__macro_private::SyscallState::Pending);
        drop(guard);

	    let low: u64;
        let high: u64;
        ::core::arch::asm!(
            "stc",
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
            in("r15") key,
            error = label { return Err(::std::io::Error::from_raw_os_error(low as isize)); }
        );

        $crate::__macro_private::Syscall::__private_new(key)
	}};
}

pub struct AsyncOwnedHandle<T: ?Sized = ()> {
	raw: RawHandle,
	_phantom: PhantomData<T>,
}

impl<T: ?Sized> Debug for AsyncOwnedHandle<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "AsyncOwnedHandle::<{}>({})", core::any::type_name::<T>(), self.raw.0)
	}
}

impl<T: ?Sized> AsHandle<T> for AsyncOwnedHandle<T> {
	fn as_handle(&self) -> BorrowedHandle<'_, T> {
		unsafe { BorrowedHandle::from_raw_handle(self.raw) }
	}
}

impl<T: ?Sized> AsRawHandle for AsyncOwnedHandle<T> {
	fn as_raw_handle(&self) -> RawHandle {
		self.raw
	}
}

impl<T: ?Sized> AsyncOwnedHandle<T> {
	pub fn from_sync(handle: OwnedHandle<T>) -> Self {
		let handle = ManuallyDrop::new(handle);
		Self {
			raw: handle.as_raw_handle(),
			_phantom: PhantomData,
		}
	}
	
	pub fn into_sync(self) -> OwnedHandle<T> {
		let this = ManuallyDrop::new(self);
		unsafe { OwnedHandle::from_raw_handle(this.raw) }
	}
}

impl<T: ProtocolTuple + ?Sized> AsyncOwnedHandle<T> {
	pub async fn new(path: impl AsRef<OsStr>, args: T::Ctor) -> io::Result<Self> {
		let path = path.as_ref().as_encoded_bytes();
		let args = T::__private_abi_convert(args);
		let uids = T::UID;

		let handle = unsafe {
			async_syscall!(1u128<<96, path.as_ptr(), path.len(), uids.as_ptr(), uids.len(), &raw const args)
		}.await?;

		Ok(AsyncOwnedHandle {
			raw: RawHandle(handle as isize),
			_phantom: PhantomData
		})
	}
}

impl<T: Protocol + ?Sized> AsyncOwnedHandle<T> {
	pub async fn new_from<U: ProtocolTuple + ?Sized>(path: impl AsRef<OsStr>, handle: OwnedHandle<U>) -> io::Result<Self> {
		let handle = ManuallyDrop::new(handle);
		let uid = T::UID;
		let path = OsStr::as_str(path.as_ref());

		let handle = unsafe {
			async_syscall!(5u128<<96, path.as_ptr(), path.len(), uid as u64, (uid >> 64) as u64, handle.as_raw_handle().0)
		}.await?;

		Ok(AsyncOwnedHandle {
			raw: RawHandle(handle as isize),
			_phantom: PhantomData
		})
	}
}

pub fn block() {
	let mut buffer = [const { MaybeUninit::<AsyncResult>::uninit() }; 32];
	let ptr = buffer.as_mut_ptr();
	let len = buffer.len();

	let mut low: usize = 0;
	let high: u64;
	unsafe {
		core::arch::asm!(
            "clc",
            "syscall",
            "jc {error}",
            inout("rax") 0u64 => low,
            out("rcx") _,
            out("rdx") high,
            inout("rdi") ptr as usize => _,
            inout("rsi") len as usize => _,
            out("r8") _,
            inout("r9") ((6u128 << 96) >> 64) as u64 => _,
            out("r10") _,
            out("r11") _,
            out("r12") _,
            error = label { panic!("{:?}", io::Error::from_raw_os_error(low as isize)); }
        );
	}

	let mut guard = SYSCALL_RESULTS.lock().unwrap();
	for i in 0..low {
		let res = unsafe { buffer[i].assume_init_ref() };
		let thing = guard.get_mut(res.key).expect("syscall key not found");
		let res = if res.error { Err(io::Error::from_raw_os_error(res.value as isize)) }
		else { Ok(res.value) };
		
		match thing {
			SyscallState::Pending => { *thing = SyscallState::Done(res); },
			SyscallState::PendingWith(_) => {
				let SyscallState::PendingWith(waker) = mem::replace(thing, SyscallState::Done(res)) else { unreachable!() };
				waker.wake();
			},
			SyscallState::Done(_) => unreachable!("kernel said already done syscall is done again")
		}
	}
}

#[repr(C)]
struct AsyncResult {
	key: usize,
	error: bool,
	value: u128,
}

pub static SYSCALL_RESULTS: Mutex<Slab<SyscallState>> = Mutex::new(Slab::new());

pub struct Syscall {
	key: usize,
}

impl Syscall {
	pub fn __private_new(key: usize) -> Self {
		Self { key }
	}
}

pub enum SyscallState {
	Pending,
	PendingWith(Waker),
	Done(io::Result<u128>),
}

impl Future for Syscall {
	type Output = io::Result<u128>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let mut guard = SYSCALL_RESULTS.lock().unwrap();
		let state = guard.get_mut(self.key).unwrap();
		*state = match state {
			SyscallState::Pending => SyscallState::PendingWith(cx.waker().clone()),
			SyscallState::PendingWith(_) => SyscallState::PendingWith(cx.waker().clone()),
			SyscallState::Done(_) => {
				let SyscallState::Done(result) = guard.remove(self.key) else { unreachable!() };
				return Poll::Ready(result);
			}
		};
		Poll::Pending
	}
}

