#![no_std]
#![no_main]

use libcool::{event, evented, io, prelude::*};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("terminal: ready");

    let mut cmd_buf = [0u8; 256];
    let mut cmd_len = 0usize;

    io::write_stdout(b"> ");

    loop {
        let _ = evented::wait_fd_read(INPUT_FD, evented::TIMEOUT_FOREVER);
        match event::read_event(INPUT_FD) {
            Ok(Some(Event::KeyChar { bytes, len })) => {
                for &c in &bytes[..len] {
                    if c == b'\n' || c == b'\r' {
                        println!();
                        if cmd_len > 0 {
                            do_command(&cmd_buf[..cmd_len]);
                            cmd_len = 0;
                        }
                        io::write_stdout(b"> ");
                    } else if c == 8 || c == 127 {
                        if cmd_len > 0 {
                            cmd_len -= 1;
                            io::write_stdout(&[8, 32, 8]);
                        }
                    } else if cmd_len < cmd_buf.len() - 1 {
                        cmd_buf[cmd_len] = c;
                        cmd_len += 1;
                        let _ = io::write_byte(io::STDOUT, c);
                    }
                }
            }
            Ok(Some(Event::MouseDown { .. })) => {}
            Ok(None) => {
                println!("\nterminal: eof");
                break;
            }
            Err(_) => {
                println!("\nterminal: read error");
                break;
            }
        }
    }

    close(INPUT_FD);
    exit(0);
}

fn do_command(cmd: &[u8]) {
    let cmd_start = cmd.iter().position(|&c| c != b' ').unwrap_or(cmd.len());
    if cmd_start == cmd.len() {
        return;
    }

    let cmd_name_end = cmd[cmd_start..]
        .iter()
        .position(|&c| c == b' ')
        .map(|end| end + cmd_start)
        .unwrap_or(cmd.len());
    let cmd_name = &cmd[cmd_start..cmd_name_end];

    if cmd_name == b"help" {
        println!("Commands: help clear echo exec info uptime abi");
    } else if cmd_name == b"clear" {
        for _ in 0..24 {
            println!();
        }
    } else if cmd_name == b"echo" {
        echo_args(cmd, cmd_name_end);
    } else if cmd_name == b"exec" {
        exec_arg(cmd, cmd_name_end);
    } else if cmd_name == b"info" {
        println!("pid={} sdk={} abi={}", getpid(), SDK_VERSION, abi_version());
    } else if cmd_name == b"uptime" {
        println!("Uptime: unavailable from userspace SDK v{}", SDK_VERSION);
    } else if cmd_name == b"abi" {
        println!(
            "coolOS ABI {} via libcool SDK {}",
            abi_version(),
            SDK_VERSION
        );
    } else {
        io::write_stdout(b"Unknown: ");
        io::write_stdout(cmd_name);
        println!();
    }
}

fn echo_args(cmd: &[u8], cmd_name_end: usize) {
    let args_start = cmd_name_end + 1;
    if args_start < cmd.len() {
        let args = &cmd[args_start..];
        let mut wrote = false;
        for &c in args {
            if c != b' ' {
                let _ = io::write_byte(io::STDOUT, c);
                wrote = true;
            } else if wrote {
                let _ = io::write_byte(io::STDOUT, b' ');
                wrote = false;
            }
        }
    }
    println!();
}

fn exec_arg(cmd: &[u8], cmd_name_end: usize) {
    let path_start = cmd_name_end + 1;
    let path_start = cmd[path_start..]
        .iter()
        .position(|&c| c != b' ')
        .map(|offset| offset + path_start)
        .unwrap_or(cmd.len());
    if path_start < cmd.len() {
        let path = &cmd[path_start..];
        if libcool::process::exec(path).is_err() {
            println!("exec failed");
        }
    } else {
        println!("usage: exec /bin/name");
    }
}
