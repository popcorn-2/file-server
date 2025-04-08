#![no_std]
#![no_main]

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
	unimplemented!()
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
