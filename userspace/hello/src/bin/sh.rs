#![no_std]
#![no_main]

use libcool::{fs, io, prelude::*};

const MAX_LINE: usize = 256;
const MAX_TOKENS: usize = 18;
const MAX_TOKEN: usize = 96;
const MAX_ARGS: usize = 7;
const MAX_PATH: usize = 160;
const LIST_BYTES: usize = 4096;

#[derive(Clone, Copy)]
struct Token {
    bytes: [u8; MAX_TOKEN],
    len: usize,
}

impl Token {
    const fn empty() -> Self {
        Self {
            bytes: [0; MAX_TOKEN],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    fn push(&mut self, byte: u8) -> bool {
        if self.len >= self.bytes.len() {
            return false;
        }
        self.bytes[self.len] = byte;
        self.len += 1;
        true
    }
}

struct Tokens {
    items: [Token; MAX_TOKENS],
    len: usize,
}

impl Tokens {
    const fn new() -> Self {
        Self {
            items: [Token::empty(); MAX_TOKENS],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.len = 0;
        for item in self.items.iter_mut() {
            item.len = 0;
        }
    }

    fn get(&self, index: usize) -> &[u8] {
        self.items[index].as_bytes()
    }
}

struct CommandSpec {
    command: usize,
    args: [usize; MAX_ARGS],
    argc: usize,
    input: Option<usize>,
    output: Option<usize>,
}

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("sh: ready abi={}", abi_version());

    let mut line = [0u8; MAX_LINE];
    let mut tokens = Tokens::new();

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
        tokens.clear();
        match parse_tokens(input, &mut tokens) {
            Ok(()) => {}
            Err(msg) => {
                println!("{}", msg);
                continue;
            }
        }
        if tokens.len == 0 {
            continue;
        }
        execute_tokens(&tokens);
    }
}

fn execute_tokens(tokens: &Tokens) {
    if let Some(pipe_idx) = find_token(tokens, b"|", 0, tokens.len) {
        run_pipeline(tokens, pipe_idx);
        return;
    }

    let has_redirection =
        find_token(tokens, b">", 0, tokens.len).is_some() || find_token(tokens, b"<", 0, tokens.len).is_some();
    if !has_redirection && run_builtin(tokens) {
        return;
    }
    run_external_range(tokens, 0, tokens.len, &[]);
}

fn run_builtin(tokens: &Tokens) -> bool {
    let cmd = tokens.get(0);
    if cmd == b"help" {
        println!("builtins: help exit clear pwd cd env echo write mkdir touch rm ls cat run abi pid sync");
        println!("syntax: command args, command > file, command < file, left | right");
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
        print_cwd();
        return true;
    }
    if cmd == b"cd" {
        let target = if tokens.len > 1 { tokens.get(1) } else { b"/" };
        match fs::chdir(target) {
            Ok(()) => {}
            Err(_) => print_err_path(b"cd", target),
        }
        return true;
    }
    if cmd == b"env" {
        io::write_stdout(b"PATH=/bin\nPWD=");
        print_cwd();
        return true;
    }
    if cmd == b"echo" {
        for idx in 1..tokens.len {
            if idx > 1 {
                io::write_stdout(b" ");
            }
            io::write_stdout(tokens.get(idx));
        }
        println!();
        return true;
    }
    if cmd == b"write" {
        if tokens.len < 3 {
            println!("usage: write <path> <text>");
        } else {
            let mut data = [0u8; 512];
            let len = join_tokens(tokens, 2, &mut data);
            match fs::write_file(tokens.get(1), &data[..len]) {
                Ok(()) => println!("write: ok"),
                Err(_) => print_err_path(b"write", tokens.get(1)),
            }
        }
        return true;
    }
    if cmd == b"mkdir" {
        one_path_op(b"mkdir", tokens, fs::create_dir);
        return true;
    }
    if cmd == b"touch" {
        one_path_op(b"touch", tokens, |path| fs::write_file(path, b""));
        return true;
    }
    if cmd == b"rm" {
        one_path_op(b"rm", tokens, fs::delete_tree);
        return true;
    }
    if cmd == b"ls" {
        let path = if tokens.len > 1 { tokens.get(1) } else { b"." };
        list_dir(path);
        return true;
    }
    if cmd == b"cat" {
        if tokens.len < 2 {
            println!("usage: cat <path>");
        } else {
            cat_file(tokens.get(1));
        }
        return true;
    }
    if cmd == b"run" {
        if tokens.len < 2 {
            println!("usage: run <path> [args...]");
        } else {
            run_external_range(tokens, 1, tokens.len, &[]);
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
    if cmd == b"sync" {
        match fs::sync() {
            Ok(()) => println!("sync: ok"),
            Err(_) => println!("sync: failed"),
        }
        return true;
    }
    false
}

fn run_pipeline(tokens: &Tokens, pipe_idx: usize) {
    if pipe_idx == 0 || pipe_idx + 1 >= tokens.len {
        println!("sh: invalid pipeline");
        return;
    }
    let Ok((read_fd, write_fd)) = io::pipe() else {
        println!("pipe: failed");
        return;
    };

    let left_maps = [(write_fd, io::STDOUT)];
    let left = spawn_external_range(tokens, 0, pipe_idx, &left_maps);
    io::close(write_fd);

    let right_maps = [(read_fd, io::STDIN)];
    let right = spawn_external_range(tokens, pipe_idx + 1, tokens.len, &right_maps);
    io::close(read_fd);

    match (left, right) {
        (Ok(left_pid), Ok(right_pid)) => {
            let _ = wait_for_exit(left_pid);
            match wait_for_exit(right_pid) {
                Ok(status) => print_status_if_failed(status),
                Err(_) => println!("wait: failed"),
            }
        }
        _ => println!("pipeline: spawn failed"),
    }
}

fn run_external_range(tokens: &Tokens, start: usize, end: usize, extra_maps: &[(u64, u64)]) {
    match spawn_external_range(tokens, start, end, extra_maps) {
        Ok(pid) => match wait_for_exit(pid) {
            Ok(status) => print_status_if_failed(status),
            Err(_) => println!("wait: failed"),
        },
        Err(_) => println!("sh: spawn failed"),
    }
}

fn spawn_external_range(
    tokens: &Tokens,
    start: usize,
    end: usize,
    extra_maps: &[(u64, u64)],
) -> Result<u64> {
    let spec = parse_command_spec(tokens, start, end)?;
    let mut path = [0u8; MAX_PATH];
    let path_len = resolve_command(tokens.get(spec.command), &mut path).ok_or(Error::Invalid)?;

    let empty: &[u8] = b"";
    let mut argv = [empty; MAX_ARGS];
    for idx in 0..spec.argc {
        argv[idx] = tokens.get(spec.args[idx]);
    }

    let mut fd_maps = [(0u64, 0u64); 4];
    let mut fd_count = 0usize;
    for &(parent_fd, child_fd) in extra_maps {
        if fd_count >= fd_maps.len() {
            return Err(Error::Invalid);
        }
        fd_maps[fd_count] = (parent_fd, child_fd);
        fd_count += 1;
    }

    let mut close_after = [0u64; 2];
    let mut close_count = 0usize;
    if let Some(input_idx) = spec.input {
        let fd = io::open(tokens.get(input_idx))?;
        if fd_count >= fd_maps.len() || close_count >= close_after.len() {
            io::close(fd);
            return Err(Error::Invalid);
        }
        fd_maps[fd_count] = (fd, io::STDIN);
        fd_count += 1;
        close_after[close_count] = fd;
        close_count += 1;
    }
    if let Some(output_idx) = spec.output {
        let fd = io::create(tokens.get(output_idx))?;
        if fd_count >= fd_maps.len() || close_count >= close_after.len() {
            io::close(fd);
            return Err(Error::Invalid);
        }
        fd_maps[fd_count] = (fd, io::STDOUT);
        fd_count += 1;
        close_after[close_count] = fd;
        close_count += 1;
    }

    let result = spawn_fds_args(&path[..path_len], &argv[..spec.argc], &fd_maps[..fd_count]);
    for &fd in &close_after[..close_count] {
        io::close(fd);
    }
    result
}

fn parse_command_spec(tokens: &Tokens, start: usize, end: usize) -> Result<CommandSpec> {
    let mut spec = CommandSpec {
        command: start,
        args: [0; MAX_ARGS],
        argc: 0,
        input: None,
        output: None,
    };
    let mut saw_command = false;
    let mut idx = start;
    while idx < end {
        let token = tokens.get(idx);
        if token == b"<" || token == b">" {
            if idx + 1 >= end {
                return Err(Error::Invalid);
            }
            if token == b"<" {
                spec.input = Some(idx + 1);
            } else {
                spec.output = Some(idx + 1);
            }
            idx += 2;
            continue;
        }
        if !saw_command {
            spec.command = idx;
            saw_command = true;
        } else {
            if spec.argc >= spec.args.len() {
                return Err(Error::Invalid);
            }
            spec.args[spec.argc] = idx;
            spec.argc += 1;
        }
        idx += 1;
    }
    if saw_command {
        Ok(spec)
    } else {
        Err(Error::Invalid)
    }
}

fn parse_tokens(line: &[u8], tokens: &mut Tokens) -> core::result::Result<(), &'static str> {
    let mut pos = 0usize;
    while pos < line.len() {
        while pos < line.len() && is_space(line[pos]) {
            pos += 1;
        }
        if pos >= line.len() {
            break;
        }
        if tokens.len >= tokens.items.len() {
            return Err("sh: too many words");
        }
        let mut token = Token::empty();
        let mut quote = 0u8;
        loop {
            if pos >= line.len() {
                break;
            }
            let mut byte = line[pos];
            if quote == 0 && is_space(byte) {
                break;
            }
            if quote == 0 && (byte == b'|' || byte == b'<' || byte == b'>') {
                if token.len == 0 {
                    token.push(byte);
                    pos += 1;
                }
                break;
            }
            if byte == b'\\' && pos + 1 < line.len() {
                pos += 1;
                byte = line[pos];
            } else if quote == 0 && (byte == b'\'' || byte == b'"') {
                quote = byte;
                pos += 1;
                continue;
            } else if quote != 0 && byte == quote {
                quote = 0;
                pos += 1;
                continue;
            }
            if !token.push(byte) {
                return Err("sh: word too long");
            }
            pos += 1;
        }
        if quote != 0 {
            return Err("sh: unterminated quote");
        }
        tokens.items[tokens.len] = token;
        tokens.len += 1;
    }
    Ok(())
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

fn resolve_command(cmd: &[u8], out: &mut [u8; MAX_PATH]) -> Option<usize> {
    if contains_byte(cmd, b'/') {
        if cmd.len() > out.len() {
            return None;
        }
        out[..cmd.len()].copy_from_slice(cmd);
        return Some(cmd.len());
    }
    let prefix = b"/bin/";
    if prefix.len() + cmd.len() > out.len() {
        return None;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    out[prefix.len()..prefix.len() + cmd.len()].copy_from_slice(cmd);
    Some(prefix.len() + cmd.len())
}

fn join_tokens(tokens: &Tokens, start: usize, out: &mut [u8]) -> usize {
    let mut len = 0usize;
    for idx in start..tokens.len {
        if idx > start {
            if len >= out.len() {
                break;
            }
            out[len] = b' ';
            len += 1;
        }
        for &byte in tokens.get(idx) {
            if len >= out.len() {
                break;
            }
            out[len] = byte;
            len += 1;
        }
    }
    len
}

fn find_token(tokens: &Tokens, needle: &[u8], start: usize, end: usize) -> Option<usize> {
    let mut idx = start;
    while idx < end {
        if tokens.get(idx) == needle {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn list_dir(path: &[u8]) {
    let mut out = [0u8; LIST_BYTES];
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

fn one_path_op(name: &[u8], tokens: &Tokens, op: fn(&[u8]) -> Result<()>) {
    if tokens.len < 2 {
        io::write_stdout(b"usage: ");
        io::write_stdout(name);
        println!(" <path>");
        return;
    }
    match op(tokens.get(1)) {
        Ok(()) => println!("ok"),
        Err(_) => print_err_path(name, tokens.get(1)),
    }
}

fn print_cwd() {
    let mut cwd = [0u8; MAX_PATH];
    match fs::getcwd(&mut cwd) {
        Ok(n) => {
            io::write_stdout(&cwd[..n]);
            println!();
        }
        Err(_) => println!("/"),
    }
}

fn print_status_if_failed(status: u64) {
    if status != 0 {
        io::write_stdout(b"exit ");
        libcool::io::write_u64(status);
        println!();
    }
}

fn wait_for_exit(pid: u64) -> Result<u64> {
    waitpid(pid)
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

fn is_space(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}
