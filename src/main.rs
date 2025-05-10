use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use anyhow::{bail, Context};

mod vfs;

fn main() -> anyhow::Result<()> {
    println!("init started...");

    let vfs_flag = Arc::new((Mutex::new(false), Condvar::new()));
    let vfs_flag2 = Arc::clone(&vfs_flag);

    let file_server = thread::spawn(move || {
        let result = vfs::vfs_main(vfs_flag2);
    });

    {
        let (lock, cvar) = &*vfs_flag;
        let mut started = lock.lock().unwrap();
        while !*started {
            started = cvar.wait(started).unwrap();
        }
    }
    drop(vfs_flag);

    println!("vfs initialised - starting root bus driver...");

    /*let exit_code = std::process::Command::new("/user/bin/shell.exec")
            .status()
            .with_context(|| "failed to spawn `/user/bin/shell.exec`")?;

    if !exit_code.success() {
        println!("`/user/bin/shell.exec` exited with error code")
    }*/
    
    let file_server_exit = file_server.join();
    
    todo!("shut down system");

    /*if exit_code.success() { Ok(()) }
    else { bail!("{:?}", exit_code.code()) }*/
}