#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("procdemo: phase33 start abi={}", abi_version());

    let parent = getpid();
    let parent_group = get_process_group(0).unwrap_or(0);
    println!("procdemo: parent pid={} pgid={}", parent, parent_group);

    let child = match spawn(b"/bin/procsleep") {
        Ok(pid) => pid,
        Err(_) => {
            println!("procdemo: spawn failed");
            exit(1);
        }
    };
    println!("procdemo: spawned child {}", child);

    if set_process_group(child, child).is_err() {
        println!("procdemo: setpgid failed");
        let _ = signal(child, Signal::Term);
        exit(2);
    }
    match get_process_group(child) {
        Ok(group) if group == child => println!("procdemo: child pgid {}", group),
        _ => {
            println!("procdemo: getpgid failed");
            let _ = signal(child, Signal::Term);
            exit(3);
        }
    }

    if signal(child, Signal::User1).is_err() {
        println!("procdemo: usr1 failed");
        let _ = signal(child, Signal::Term);
        exit(4);
    }
    println!("procdemo: usr1 ok");

    if signal(child, Signal::Stop).is_err() {
        println!("procdemo: stop failed");
        let _ = signal(child, Signal::Term);
        exit(5);
    }
    println!("procdemo: stop ok");

    sleep_ms(20);

    if signal(child, Signal::Continue).is_err() {
        println!("procdemo: cont failed");
        let _ = signal(child, Signal::Term);
        exit(6);
    }
    println!("procdemo: cont ok");

    match signal_group(child, Signal::Term) {
        Ok(count) => println!("procdemo: group term count={}", count),
        Err(_) => {
            println!("procdemo: group term failed");
            let _ = signal(child, Signal::Term);
            exit(7);
        }
    }

    match wait_for_exit(child) {
        Ok(code) => println!("procdemo: wait exit {}", code),
        Err(_) => {
            println!("procdemo: wait failed");
            exit(8);
        }
    }

    println!("procdemo: phase33 ok");
    exit(0);
}

fn wait_for_exit(pid: u64) -> Result<u64> {
    let _ = wait_child(pid, libcool::evented::TIMEOUT_FOREVER);
    waitpid(pid)
}
