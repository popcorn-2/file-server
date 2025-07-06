#![feature(ergonomic_clones)]
#![feature(debug_closure_helpers)]
#![feature(popcorn_protocol)]
#![feature(bstr)]
#![feature(slice_ptr_get)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::ffi::OsStr;
use std::fs::File;
use std::mem::ManuallyDrop;
use std::io::Read as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use anyhow::Context;
use std::os::popcorn::process::CommandExt;
use std::os::popcorn::handle::{AsRawHandle, FromRawHandle};
use std::os::popcorn::proto::io::Read;
use std::ptr::NonNull;
use std::os::popcorn::ffi::OsStrExt;

mod vfs;

fn main() -> anyhow::Result<()> {
    println!("init started...");

    let ramdisk = {
        let handle = std::os::popcorn::env::get_handle::<Read>("popcorn.init.ramdisk")
                .context("no ramdisk handle provided")?;
        
        // SAFETY: The `File` object doesn't get dropped, so we don't call close on a `BorrowedHandle`
        // The `BorrowedHandle` has a static lifetime so the handle is always valid
        let mut file = ManuallyDrop::new(unsafe { File::from_raw_handle(handle.as_raw_handle()) });
        let mut buf = vec![];
        let bytes = file.read_to_end(&mut buf)
                .context("failed to read ramdisk")?;
        println!("read {bytes} bytes");
        buf
    };
    
    let vfs_flag = Arc::new(AtomicBool::new(false));
    let vfs_flag2 = Arc::clone(&vfs_flag);

    let file_server = thread::Builder::new()
            .name("vfs".to_owned())
            .spawn(move || {
                let result = vfs::vfs_main(vfs_flag2, &ramdisk);
            })
            .expect("unable to start vfs");
    
    while !vfs_flag.load(Ordering::Relaxed) { thread::yield_now(); }

    drop(vfs_flag);

    println!("vfs initialised - starting root bus driver...");

    let root_driver = format!("fs:/system/bin/driver/{}_root.exec", std::env::consts::ARCH);
    let exit_code = std::process::Command::new(&root_driver)
            .stdin(std::process::Stdio::null())
            .handle(OsStr::from_str("popcorn.init.root-bus-descriptor").to_owned(), std::os::popcorn::process::inherit())
            .spawn()
            .with_context(|| format!("failed to spawn `{root_driver}`"))?;
    
    loop { thread::yield_now(); }
}
