#![feature(popcorn_protocol)]
#![feature(popcorn_std)]
#![feature(asm_goto_with_outputs)]

use std::task::{Context, Poll, Waker};
use async_executor::LocalExecutor;
use std::future::Future;
use std::pin::pin;

pub mod io;

thread_local! {
	static RT: LocalExecutor = const { LocalExecutor::new() };
}

pub fn block_on<T>(f: impl Future<Output = T>) -> T {
	RT.with(|rt| {
		let mut f = pin!(rt.run(f));
		loop {
			match f.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
				Poll::Ready(val) => return val,
				Poll::Pending => {
					io::popcorn::block();
					rt.try_tick();
				}
			}
		}
	})
}

pub fn spawn(f: impl Future<Output = ()> + 'static) {
	RT.with(|rt| {
		rt.spawn(f).detach();
	})
}
