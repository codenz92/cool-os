#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

const MAX_LINE: usize = 256;
const MAX_WORDS: usize = 8;
const MAX_PATH: usize = 160;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("sh: ready abi={}", abi_version());

    let mut cwd = [0u8; MAX_PATH];
    cwd[0] = b'/';
    let mut cwd_len = 1usize;
    let mut line = [0u8; MAX_LINE];
    let mut words = [(0usize, 0usize); MAX_WORDS];

    loop {
        io::write_stdout(b"$ ");
        let n = match io::read(io::STDIN, &mut line) {
            Ok(0) => {
                println!("sh: eof");
                exit(0);
            }
            Ok(n) => n,
            Err(_) => {
                println!("sh: read failed");
                exit(1);
            }
        };
        let input = trim_line(&line[..n]);
        let argc = split_words(input, &mut words);
        if argc == 0 {
            continue;
        }
        if run_builtin(input, argc, &words, &mut cwd, &mut cwd_len) {
            continue;
        }
        run_external(input, argc, &words, &cwd[..cwd_len], false);
    }
}

fn run_builtin(
    line: &[u8],
    argc: usize,
    words: &[(usize, usize); MAX_WORDS],
    cwd: &mut [u8; MAX_PATH],
    cwd_len: &mut usize,
) -> bool {
    let cmd = word(line, words, 0);
    if cmd == b"help" {
        println!("builtins: help exit clear pwd cd ls cat echo write mkdir touch rm run abi pid");
        println!("external commands: type a name for /bin/name, or run <path> [args...]");
        return true;
    }
    if cmd == b"exit" {
        exit(0);
    }
    if cmd == b"clear" {
        for _ in 0..28 {
            println!();
        }
        return true;
    }
    if cmd == b"pwd" {
        io::write_stdout(&cwd[..*cwd_len]);
        println!();
        return true;
    }
    if cmd == b"cd" {
        let target = if argc > 1 { word(line, words, 1) } else { b"/" };
        let mut path = [0u8; MAX_PATH];
        if let Some(path_len) = resolve_path(&cwd[..*cwd_len], target, &mut path) {
            let mut listing = [0u8; 1];
            if fs::list_dir(&path[..path_len], &mut listing).is_ok() {
                cwd[..path_len].copy_from_slice(&path[..path_len]);
                *cwd_len = path_len;
            } else {
                print_err_path(b"cd", &path[..path_len]);
            }
        } else {
            println!("cd: path too long");
        }
        return true;
    }
    if cmd == b"ls" {
        let target = if argc > 1 {
            word(line, words, 1)
        } else {
            &cwd[..*cwd_len]
        };
        let mut path = [0u8; MAX_PATH];
        if let Some(path_len) = resolve_path(&cwd[..*cwd_len], target, &mut path) {
            list_dir(&path[..path_len]);
        } else {
            println!("ls: path too long");
        }
        return true;
    }
    if cmd == b"cat" {
        if argc < 2 {
            println!("usage: cat <path>");
        } else {
            let mut path = [0u8; MAX_PATH];
            if let Some(path_len) = resolve_path(&cwd[..*cwd_len], word(line, words, 1), &mut path)
            {
                cat_file(&path[..path_len]);
            } else {
                println!("cat: path too long");
            }
        }
        return true;
    }
    if cmd == b"echo" {
        for idx in 1..argc {
            if idx > 1 {
                io::write_stdout(b" ");
            }
            io::write_stdout(word(line, words, idx));
        }
        println!();
        return true;
    }
    if cmd == b"write" {
        if argc < 3 {
            println!("usage: write <path> <text>");
        } else {
            let mut path = [0u8; MAX_PATH];
            if let Some(path_len) = resolve_path(&cwd[..*cwd_len], word(line, words, 1), &mut path)
            {
                let text_start = words[2].0;
                match fs::write_file(&path[..path_len], &line[text_start..]) {
                    Ok(()) => println!("write: ok"),
                    Err(_) => print_err_path(b"write", &path[..path_len]),
                }
            } else {
                println!("write: path too long");
            }
        }
        return true;
    }
    if cmd == b"mkdir" {
        one_path_op(
            b"mkdir",
            argc,
            line,
            words,
            &cwd[..*cwd_len],
            fs::create_dir,
        );
        return true;
    }
    if cmd == b"touch" {
        one_path_op(b"touch", argc, line, words, &cwd[..*cwd_len], |path| {
            fs::write_file(path, b"")
        });
        return true;
    }
    if cmd == b"rm" {
        one_path_op(b"rm", argc, line, words, &cwd[..*cwd_len], fs::delete_tree);
        return true;
    }
    if cmd == b"run" {
        if argc < 2 {
            println!("usage: run <path> [args...]");
        } else {
            run_external_from(line, argc, words, &cwd[..*cwd_len], 1, true);
        }
        return true;
    }
    if cmd == b"abi" {
        println!("abi={}", abi_version());
        return true;
    }
    if cmd == b"pid" {
        println!("pid={}", getpid());
        return true;
    }
    false
}

fn run_external(
    line: &[u8],
    argc: usize,
    words: &[(usize, usize); MAX_WORDS],
    cwd: &[u8],
    first_word_is_path: bool,
) {
    run_external_from(line, argc, words, cwd, 0, first_word_is_path);
}

fn run_external_from(
    line: &[u8],
    argc: usize,
    words: &[(usize, usize); MAX_WORDS],
    cwd: &[u8],
    start_word: usize,
    first_word_is_path: bool,
) {
    let command = word(line, words, start_word);
    let mut path = [0u8; MAX_PATH];
    let path_len = if first_word_is_path || contains_byte(command, b'/') {
        resolve_path(cwd, command, &mut path)
    } else {
        resolve_bin(command, &mut path)
    };
    let Some(path_len) = path_len else {
        println!("sh: path too long");
        return;
    };

    let empty: &[u8] = b"";
    let mut argv = [empty; MAX_WORDS - 1];
    let arg_count = argc.saturating_sub(start_word + 1);
    for idx in 0..arg_count {
        argv[idx] = word(line, words, start_word + 1 + idx);
    }
    match spawn_args(&path[..path_len], &argv[..arg_count]) {
        Ok(pid) => match wait_for_exit(pid) {
            Ok(status) => {
                if status != 0 {
                    io::write_stdout(b"exit ");
                    libcool::io::write_u64(status);
                    println!();
                }
            }
            Err(_) => println!("wait: failed"),
        },
        Err(_) => {
            io::write_stdout(b"not found: ");
            io::write_stdout(&path[..path_len]);
            println!();
        }
    }
}

fn wait_for_exit(pid: u64) -> Result<u64> {
    waitpid(pid)
}

fn trim_line(mut line: &[u8]) -> &[u8] {
    while let Some((&last, rest)) = line.split_last() {
        if last == b'\n' || last == b'\r' {
            line = rest;
        } else {
            break;
        }
    }
    line
}

fn split_words(line: &[u8], out: &mut [(usize, usize); MAX_WORDS]) -> usize {
    let mut count = 0usize;
    let mut pos = 0usize;
    while pos < line.len() && count < out.len() {
        while pos < line.len() && line[pos] == b' ' {
            pos += 1;
        }
        if pos >= line.len() {
            break;
        }
        let start = pos;
        while pos < line.len() && line[pos] != b' ' {
            pos += 1;
        }
        out[count] = (start, pos);
        count += 1;
    }
    count
}

fn word<'a>(line: &'a [u8], words: &[(usize, usize); MAX_WORDS], idx: usize) -> &'a [u8] {
    let (start, end) = words[idx];
    &line[start..end]
}

fn resolve_bin(cmd: &[u8], out: &mut [u8; MAX_PATH]) -> Option<usize> {
    let prefix = b"/bin/";
    if prefix.len() + cmd.len() > out.len() {
        return None;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    out[prefix.len()..prefix.len() + cmd.len()].copy_from_slice(cmd);
    Some(prefix.len() + cmd.len())
}

fn resolve_path(cwd: &[u8], arg: &[u8], out: &mut [u8; MAX_PATH]) -> Option<usize> {
    if arg.starts_with(b"/") {
        if arg.len() > out.len() {
            return None;
        }
        out[..arg.len()].copy_from_slice(arg);
        return Some(arg.len());
    }
    if cwd == b"/" {
        if arg.len() + 1 > out.len() {
            return None;
        }
        out[0] = b'/';
        out[1..1 + arg.len()].copy_from_slice(arg);
        return Some(1 + arg.len());
    }
    if cwd.len() + 1 + arg.len() > out.len() {
        return None;
    }
    out[..cwd.len()].copy_from_slice(cwd);
    out[cwd.len()] = b'/';
    out[cwd.len() + 1..cwd.len() + 1 + arg.len()].copy_from_slice(arg);
    Some(cwd.len() + 1 + arg.len())
}

fn list_dir(path: &[u8]) {
    let mut out = [0u8; 2048];
    match fs::list_dir(path, &mut out) {
        Ok(n) => {
            if n == 0 {
                println!();
            } else {
                io::write_stdout(&out[..n]);
            }
        }
        Err(_) => print_err_path(b"ls", path),
    }
}

fn cat_file(path: &[u8]) {
    match io::File::open(path) {
        Ok(file) => {
            let mut buf = [0u8; 512];
            loop {
                match file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = io::write_all(io::STDOUT, &buf[..n]);
                    }
                    Err(_) => {
                        print_err_path(b"cat", path);
                        break;
                    }
                }
            }
            file.close();
        }
        Err(_) => print_err_path(b"cat", path),
    }
}

fn one_path_op(
    name: &[u8],
    argc: usize,
    line: &[u8],
    words: &[(usize, usize); MAX_WORDS],
    cwd: &[u8],
    op: fn(&[u8]) -> Result<()>,
) {
    if argc < 2 {
        io::write_stdout(b"usage: ");
        io::write_stdout(name);
        println!(" <path>");
        return;
    }
    let mut path = [0u8; MAX_PATH];
    if let Some(path_len) = resolve_path(cwd, word(line, words, 1), &mut path) {
        match op(&path[..path_len]) {
            Ok(()) => println!("ok"),
            Err(_) => print_err_path(name, &path[..path_len]),
        }
    } else {
        io::write_stdout(name);
        println!(": path too long");
    }
}

fn print_err_path(prefix: &[u8], path: &[u8]) {
    io::write_stdout(prefix);
    io::write_stdout(b": failed: ");
    io::write_stdout(path);
    println!();
}

fn contains_byte(bytes: &[u8], needle: u8) -> bool {
    for &byte in bytes {
        if byte == needle {
            return true;
        }
    }
    false
}
