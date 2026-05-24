use std::{io, mem};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::os::popcorn::io::{PopcornAsyncHandle, AsHandle, IntoRawHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::os::popcorn::proto::ProtocolList;
use std::pin::Pin;
use std::sync::{LazyLock, Mutex};
use std::task::{Context, Poll, Waker};
use slab::Slab;

pub struct AsyncOwnedHandle<T = ()> {
	raw: RawHandle,
	_phantom: PhantomData<T>,
}

impl<T> Debug for AsyncOwnedHandle<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "AsyncOwnedHandle::<{}>({})", core::any::type_name::<T>(), self.raw.0)
	}
}

impl<T: ProtocolList> AsHandle<T::InvertAsync> for AsyncOwnedHandle<T> {
	fn as_handle(&self) -> BorrowedHandle<'_, T::InvertAsync> {
		unsafe { BorrowedHandle::borrow_raw(self.raw) }
	}
}

impl<T> AsRawHandle for AsyncOwnedHandle<T> {
	fn as_raw_handle(&self) -> RawHandle {
		self.raw
	}
}

impl<T> IntoRawHandle for AsyncOwnedHandle<T> {
	fn into_raw_handle(self) -> RawHandle {
        let this = ManuallyDrop::new(self);
		this.raw
	}
}

impl<T: ProtocolList> AsyncOwnedHandle<T> {
	pub fn from_sync(handle: OwnedHandle<T::InvertAsync>) -> Self {
		Self {
			raw: handle.into_raw_handle(),
			_phantom: PhantomData,
		}
	}
	
	pub fn into_sync(self) -> OwnedHandle<T::InvertAsync> {
		unsafe { OwnedHandle::from_raw_handle(self.into_raw_handle()) }
	}
}

/*
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
}*/

impl<T> PopcornAsyncHandle for AsyncOwnedHandle<T> {
    type Protocols = T;

    fn wait_result(f: impl FnOnce(usize) -> io::Result<u128>) -> impl Future<Output = io::Result<u128>> {
        let mut guard = SYSCALL_RESULTS.lock().unwrap();
        let key = guard.insert(SyscallState::Pending);
        drop(guard);

        async move {
            match f(key) {
                Ok(_) => Syscall { key }.await,
                e => core::future::ready(e).await,
            }
        }
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

		// key 0 indicates a nop event
		if res.key == 0 { continue; }

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

pub static SYSCALL_RESULTS: LazyLock<Mutex<Slab<SyscallState>>> = LazyLock::new(|| {
	let mut slab = Slab::new();
	slab.insert(SyscallState::Pending);
	Mutex::new(slab)
});

pub struct Syscall {
	key: usize,
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

