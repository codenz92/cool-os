#![no_std]
#![no_main]

use libcool::{evented, io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    match run() {
        Ok(()) => exit(0),
        Err(_) => {
            println!("polldemo: failed");
            exit(1);
        }
    }
}

fn run() -> Result<()> {
    println!("polldemo: abi={}", abi_version());

    let (read_fd, write_fd) = io::pipe()?;
    let mut desc = [evented::PollDesc::fd_read(read_fd)];
    let ready = evented::poll(&mut desc, 10)?;
    if ready != 0 || desc[0].revents != 0 {
        return Err(Error::Failed);
    }
    println!("polldemo: timeout ok");

    io::write_all(write_fd, b"poll pipe ok\n")?;
    desc[0] = evented::PollDesc::fd_read(read_fd);
    let ready = evented::poll(&mut desc, evented::TIMEOUT_FOREVER)?;
    if ready != 1 || !desc[0].is_ready(evented::READ) {
        return Err(Error::Failed);
    }
    let mut buf = [0u8; 16];
    let n = io::read(read_fd, &mut buf)?;
    if n != b"poll pipe ok\n".len() || &buf[..n] != b"poll pipe ok\n" {
        return Err(Error::Failed);
    }
    io::close(read_fd);
    io::close(write_fd);
    println!("polldemo: pipe ok");

    let child = spawn(b"/bin/hello")?;
    let mut child_desc = [evented::PollDesc::child(child)];
    let ready = evented::poll(&mut child_desc, evented::TIMEOUT_FOREVER)?;
    if ready != 1 || child_desc[0].revents & evented::CHILD == 0 {
        return Err(Error::Failed);
    }
    let status = waitpid(child)?;
    if status != 0 {
        return Err(Error::Failed);
    }
    println!("polldemo: child ok");
    println!("polldemo: done");
    Ok(())
}
