mod private {
	use ::core::arch::asm;
	use core::simd::u64x2;

	#[diagnostic::on_unimplemented(
		message = "cannot pass non-Protocol trait to `create!()`"
	)]
	pub trait Protocol {
		fn as_raw_handle(&self) -> usize;

		fn dispatch(&self, method: u128, b: usize, c: usize, d: usize) -> isize {
			unsafe { dispatch_shim(method, self.as_raw_handle(), b, c, d) }
		}
	}

	pub unsafe fn dispatch_shim(method: u128, a: usize, b: usize, c: usize, d: usize) -> isize {
		let out: isize;
		let method = u64x2::from_array([
			method as u64,
			(method >> 64) as u64
		]);
		unsafe {
			asm!(
				//"movaps xmm0, [{0}]",
				"syscall",
				in("xmm0") method,
				lateout("rcx") _,
				lateout("r11")_,
				in("rdi") a,
				in("rsi") b,
				in("rdx") c,
				in("r8") d,
				lateout("rax") out,
			);
		}
		out
	}
}

const fn proto_method_from_parts(protocol_id: u128, method_id: u32) -> u128 {
	const PROTO_MASK: u128 = 0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF;
	protocol_id | ((method_id as u128) << PROTO_MASK.trailing_ones())
}

pub mod core {
	pub mod object {
		pub trait Object: super::super::private::Protocol {
			fn destroy(&self) {
				self.dispatch(
					super::super::proto_method_from_parts(0, 1),
					0,
					0,
					0
				);
			}
		}

		pub trait ObjectReceiver {

		}
	}

	pub mod server {
		pub struct SyncPacketHandler<T: Sync> {
			handle: T,
			buffer: [MaybeUninit<u8>; size_of::<Packet>() * 8],
		}

		impl<T: Sync> SyncPacketHandler<T> {
			pub fn new(handle: T) -> Self {
				Self {
					handle,
					buffer: [const { MaybeUninit::uninit() }; size_of::<Packet>() * 8],
				}
			}

			pub fn register_handler(&mut self) -> &mut Self {
				self
			}

			pub fn main_loop(&mut self) -> Result<!, usize> {
				let mut buf = BorrowedBuf::from(self.buffer);

				loop {
					self.handle.next(&mut buf)?;
					let packet_buf = unsafe {
						&*core::ptr::slice_from_raw_parts(
							buf.filled().as_ptr().cast::<Packet>(),
							buf.filled().len() / size_of::<Packet>()
						)
					};

					for packet in packet_buf {

					}

					buf.clear();
				}
			}
		}

		use core::io::BorrowedBuf;
		use core::mem::MaybeUninit;

		#[repr(C)]
		struct Packet {
			proto_method: u128,
			a: u64,
			b: u64,
			c: u64,
			d: u64,
		}

		pub trait Sync: super::super::private::Protocol {
			fn next(&self, packet_buffer: &mut BorrowedBuf) -> Result<(), usize> {
				let mut cursor = packet_buffer.unfilled();
				let res = self.dispatch(
					super::super::proto_method_from_parts(1, 0),
					0,
					cursor.uninit_mut().as_ptr() as _,
					cursor.uninit_mut().len()
				);
				if res < 0 { Err((-res) as usize) }
				else {
					unsafe {
						cursor.set_init(res as usize);
					}
					cursor.advance(res as usize);
					Ok(())
				}
			}

			fn reply(&self, packets: &[Packet])
		}
	}

	pub mod io {
		pub trait Write: super::super::private::Protocol {

		}

		pub trait Read: super::super::private::Protocol {

		}
	}
}

pub(crate) macro create {
    ($path:literal $(, impl $tr:path $(| $tr2:path)*)?) => {{
	    struct _Test<T: private::Protocol>(::core::marker::PhantomData<T>);
	    $({
		    struct _Test2<T: $tr>(_Test<T>);
	    }
	    $({
		    struct _Test2<T: $tr2>(_Test<T>);
	    })*)?

		fn shim() -> Result<impl core::object::Object $(+ $tr $(+ $tr2)*)?, usize> {
			struct Handle(usize);

			impl private::Protocol for Handle {
				fn as_raw_handle(&self) -> usize {
			        self.0
                }
			}

			impl core::object::Object for Handle {}
			$(impl $tr for Handle {}
			$(impl $tr2 for Handle {})*)?

			impl Drop for Handle {
				fn drop(&mut self) {
			        core::object::Object::destroy(self);
			    }
			}

			let path: &str = $path;
			let res = unsafe { private::dispatch_shim(
				0,
				path.as_ptr() as usize,
				path.len(),
				0,
				0,
			) };

		    if res >= 0 {
				Ok(Handle(res as usize))
			} else {
				Err((-res) as usize)
			}
		}

	    shim()
    }}
}
