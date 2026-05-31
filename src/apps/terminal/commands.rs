impl TerminalApp {
    fn run_command(&mut self, input: &str) {
        let mut words = input.split_whitespace();
        self.set_fg(FG_OUTPUT);
        match words.next() {
            Some("help") => self.cmd_help(),

            Some("clear") => {
                self.fill_background();
                self.col = 0;
                self.row = 0;
            }

            Some("reboot") => crate::interrupts::reboot(),

            Some("echo") => {
                for word in words {
                    self.print_str(word);
                    self.print_char(' ');
                }
                self.print_char('\n');
            }

            Some("pwd") => {
                self.set_fg(FG_DIR);
                let cwd = self.cwd.clone();
                self.print_str(&cwd);
                self.print_char('\n');
            }

            Some("cd") => {
                let target = match words.next() {
                    Some(p) => resolve_path(&self.cwd, p),
                    None => String::from("/"),
                };
                if crate::vfs::vfs_list_dir(&target).is_some() {
                    self.cwd = target;
                } else {
                    self.set_fg(FG_ERROR);
                    self.print_str("cd: no such directory\n");
                }
            }

            Some("ls") => {
                let path_arg = words.next();
                let path = match path_arg {
                    Some(p) => resolve_path(&self.cwd, p),
                    None => self.cwd.clone(),
                };
                self.cmd_ls(&path);
            }

            Some("touch") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_touch(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: touch <path>\n");
                }
            },

            Some("mkdir") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_mkdir(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: mkdir <path>\n");
                }
            },

            Some("cat") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_cat(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: cat <path>\n");
                }
            },

            Some("hash") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_hash(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: hash <path>\n");
                }
            },

            Some("write") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    let text = collect_words(words);
                    self.cmd_write_file(&path, &text);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: write <path> <text>\n");
                }
            },

            Some("rm") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_rm(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: rm <path>\n");
                }
            },

            Some("ps") => self.cmd_ps(),

            Some("kill") => match words.next().and_then(parse_usize) {
                Some(pid) => self.cmd_kill(pid),
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: kill <pid>\n");
                }
            },

            Some("wait") => match words.next().and_then(parse_usize) {
                Some(pid) => self.cmd_wait(pid),
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: wait <pid>\n");
                }
            },

            Some("reap") => self.cmd_reap(),

            Some("info") => self.cmd_info(),

            Some("uptime") => self.cmd_uptime(),

            Some("devices") => self.cmd_devices(),

            Some("net") => self.cmd_lines("NETWORK", crate::net::status_lines()),

            Some("netproto") => self.cmd_lines("NETWORK PROTOCOLS", crate::net::protocol_lines()),

            Some("tlscheck") => {
                let lines = crate::tls::selftest_lines();
                for line in &lines {
                    crate::println!("{}", line);
                }
                self.cmd_lines("TLS HOSTNAME CHECK", lines);
            }

            Some("netapi") => {
                self.cmd_lines("NETWORK API SETTINGS", crate::settings_state::lines())
            }

            Some(cmd @ ("http" | "https")) => {
                let host = words.next();
                let path = words.next().unwrap_or("/");
                match host {
                    Some(host) => self.cmd_http(cmd, host, path),
                    None => {
                        self.set_fg(FG_ERROR);
                        self.print_str("usage: http|https <host-or-url> [path]\n");
                    }
                }
            }

            Some("dns") => match words.next() {
                Some(host) => match crate::net::dns_resolve(host) {
                    Ok(addr) => self.cmd_lines(
                        "DNS",
                        alloc::vec![alloc::format!(
                            "{} -> {}",
                            host,
                            crate::net::ipv4_string(addr)
                        )],
                    ),
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("dns: ");
                        self.print_str(err);
                        self.print_char('\n');
                    }
                },
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: dns <host>\n");
                }
            },

            Some("ping") => match words.next() {
                Some(host) => match crate::net::dns_resolve(host).and_then(crate::net::icmp_ping) {
                    Ok(()) => self.cmd_lines("PING", alloc::vec![alloc::format!("{} ok", host)]),
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("ping: ");
                        self.print_str(err);
                        self.print_char('\n');
                    }
                },
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: ping <host-or-ip>\n");
                }
            },

            Some("power") => self.cmd_power(words.next()),

            Some("log") => self.cmd_log(),

            Some("logs") => self.cmd_lines("LOG VIEW", crate::klog::lines()),

            Some("diagnostics") | Some("diag") => {
                self.cmd_lines("DIAGNOSTICS", diagnostics_lines())
            }

            Some("engine") | Some("browser-engine") => self.cmd_engine(words.next()),

            Some("sysreport") => self.cmd_sysreport(words.next()),

            Some("devkit") => self.cmd_devkit(),

            Some("profiler") => {
                let mut lines = crate::profiler::lines();
                lines.extend(crate::boot_watchdog::lines());
                lines.extend(crate::boot_health::status_lines());
                lines.extend(crate::deferred::lines());
                self.cmd_lines("BOOT/SESSION PROFILER", lines);
            }

            Some("compositor") | Some("smoothness") => {
                self.cmd_lines("COMPOSITOR", crate::wm::compositor::compositor_lines())
            }

            Some("heap") => self.cmd_lines("HEAP DIAGNOSTICS", crate::allocator::heap_lines()),

            Some("memory") | Some("mem") => {
                let mut lines = crate::memory_pressure::lines();
                push_terminal_section(
                    &mut lines,
                    "task memory",
                    crate::scheduler::task_memory_lines(),
                );
                self.cmd_lines("MEMORY PRESSURE", lines)
            }

            Some("slab") => self.cmd_lines("SLAB DIAGNOSTICS", crate::slab::lines()),

            Some("waitq") => self.cmd_lines("WAIT QUEUES", crate::wait_queue::lines()),

            Some("writeback") => self.cmd_lines("WRITEBACK", crate::writeback::lines()),

            Some("selftest") => self.cmd_lines("SELFTEST", crate::selftest::lines()),

            Some("font") => self.cmd_lines("FONT RENDERER", crate::font::lines()),

            Some("deferred") => self.cmd_lines("DEFERRED WORK", crate::deferred::lines()),

            Some("tasksnap") => self.cmd_lines("TASK SNAPSHOT", crate::task_snapshot::lines()),

            Some("fsck") => self.cmd_fsck(),

            Some("coolfs") => self.cmd_lines("COOLFS", crate::coolfs::lines()),

            Some("fsrepair") => self.cmd_lines("FS REPAIR", crate::fs_hardening::repair()),

            Some("recovery") => self.cmd_recovery(words.collect()),

            Some("update") => self.cmd_update(words.collect()),

            Some("boot") => self.cmd_boot(words.collect()),

            Some("mounts") => self.cmd_lines("MOUNTS", crate::fs_hardening::status_lines()),

            Some("vfs") => self.cmd_lines("VFS", crate::vfs::mount_lines()),

            Some("path") => match words.next() {
                Some(path) => {
                    let path = resolve_path(&self.cwd, path);
                    self.cmd_lines("PATH", crate::vfs::path_lines(&[&path]));
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: path <file-or-dir>\n");
                }
            },

            Some("perm") => match words.next() {
                Some(path) => {
                    let path = resolve_path(&self.cwd, path);
                    self.cmd_perm(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: perm <path>\n");
                }
            },

            Some("chmod") => match (words.next(), words.next()) {
                (Some(mode), Some(path)) => {
                    let path = resolve_path(&self.cwd, path);
                    self.cmd_chmod(mode, &path);
                }
                _ => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: chmod <mode> <path>\n");
                }
            },

            Some("chown") => match (words.next(), words.next()) {
                (Some(owner), Some(path)) => {
                    let path = resolve_path(&self.cwd, path);
                    self.cmd_chown(owner, &path);
                }
                _ => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: chown <uid>[:gid] <path>\n");
                }
            },

            Some("journal") => self.cmd_lines("FS JOURNAL", crate::fs_hardening::journal_lines()),

            Some("flush") => self.print_result(
                "flush",
                crate::writeback::barrier().map_err(|_| "flush failed"),
            ),

            Some("df") => self.cmd_df(),

            Some("shortcuts") => self.cmd_lines("SHORTCUTS", crate::shortcuts::summary_lines()),

            Some("icons") => self.cmd_lines("DESKTOP ICONS", crate::desktop_settings::icon_lines()),

            Some("access") => self.cmd_access(words.next(), words.next()),

            Some("apps") => self.cmd_lines("APP LIFECYCLE", crate::app_lifecycle::lines()),

            Some("appcats") => {
                self.cmd_lines("APP CATEGORIES", crate::app_metadata::category_lines())
            }

            Some("pinned") => self.cmd_pinned(words.collect()),

            Some("pin") => self.cmd_pin(collect_words(words)),

            Some("unpin") => self.cmd_unpin(collect_words(words)),

            Some("startmenu") => self.cmd_startmenu(words.next()),

            Some("recent") => self.cmd_recent(),

            Some("startup") => self.cmd_startup(words.collect()),

            Some("search") => {
                let query = collect_words(words);
                if query.is_empty() {
                    self.cmd_lines("SEARCH INDEX", crate::search_index::lines(None));
                } else {
                    self.cmd_lines("SEARCH", crate::search_index::lines(Some(&query)));
                }
            }

            Some("index") => {
                crate::search_index::refresh();
                self.cmd_lines("SEARCH INDEX", crate::search_index::lines(None));
            }

            Some("drivers") => {
                crate::drivers::refresh();
                self.cmd_lines("DRIVERS", crate::drivers::lines());
            }

            Some("whoami") => self.cmd_whoami(),

            Some("id") => self.cmd_id(words.next()),

            Some("groups") => self.cmd_groups(words.next()),

            Some("login") | Some("su") => self.cmd_login(words.next(), words.next()),

            Some("lock") => self.cmd_lock(),

            Some("logout") => self.cmd_logout(),

            Some("passwd") => self.cmd_passwd(words.next(), words.next()),

            Some("setup") => self.cmd_setup(words.next(), words.next()),

            Some("install") => self.cmd_install(words.collect()),

            Some("account") => self.cmd_account(words.collect()),

            Some("umask") => self.cmd_umask(words.next()),

            Some("users") => self.cmd_lines("USERS", crate::security::lines()),

            Some("security") => self.cmd_lines("SECURITY", crate::security::lines()),

            Some("pkg") => {
                let op = words.next();
                let arg = words.next();
                let rest: Vec<&str> = words.collect();
                self.cmd_pkg(op, arg, rest);
            }

            Some("proc") => self.cmd_lines("PROCESS MODEL", crate::process_model::status_lines()),

            Some("zombies") => {
                self.cmd_lines("ZOMBIE POLICY", crate::process_model::zombie_policy_lines())
            }

            Some("signal") => self.cmd_signal(words.next(), words.next()),

            Some("pgroup") => self.cmd_pgroup(words.next(), words.next()),

            Some("tty") => self.cmd_tty(),

            Some("events") => self.cmd_lines("EVENTS", crate::event_bus::lines(12)),

            Some("jobs") => self.cmd_lines("JOBS", crate::jobs::lines()),

            Some("job") => self.cmd_job(words.collect()),

            Some("fg") => self.cmd_fg(words.next()),

            Some("bg") => self.cmd_bg(words.next()),

            Some("services") => self.cmd_services(words.next(), words.next()),

            Some("crash") => self.cmd_lines("CRASH DUMP", crate::crashdump::detailed_lines()),

            Some("abi") => self.cmd_lines("ABI", crate::abi::lines()),

            Some("notify") => self.cmd_notify(words.next(), words.next()),

            Some("screenshot") => {
                let path = words.next().unwrap_or("/LOGS/WINDOW.PPM");
                crate::wm::request_focused_screenshot(path);
                self.set_fg(FG_ACCENT);
                self.print_str("queued screenshot ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }

            Some("clip") => {
                let mut text = String::new();
                for word in words {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(word);
                }
                if text.is_empty() {
                    self.set_fg(FG_ACCENT);
                    self.print_str("clipboard: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(&crate::clipboard::summary());
                    self.print_str(" [");
                    self.print_str(crate::clipboard::mime_type());
                    self.print_str("]");
                    self.print_char('\n');
                } else {
                    crate::clipboard::set_text(&text);
                    self.set_fg(FG_ACCENT);
                    self.print_str("copied text\n");
                }
            }

            Some("clipmimes") => {
                self.cmd_lines("CLIPBOARD MIME TYPES", crate::clipboard::mime_lines())
            }

            Some("clipimg") => match (words.next(), words.next()) {
                (Some(w), Some(h)) => {
                    let width = parse_usize(w).unwrap_or(16).min(64);
                    let height = parse_usize(h).unwrap_or(16).min(64);
                    let mut pixels = Vec::new();
                    pixels.resize(width.saturating_mul(height).saturating_mul(4), 0u8);
                    for y in 0..height {
                        for x in 0..width {
                            let idx = (y * width + x) * 4;
                            let hot = ((x / 4) + (y / 4)) % 2 == 0;
                            pixels[idx] = if hot { 0x00 } else { 0x22 };
                            pixels[idx + 1] = if hot { 0xbb } else { 0x44 };
                            pixels[idx + 2] = if hot { 0xff } else { 0x88 };
                            pixels[idx + 3] = 0xff;
                        }
                    }
                    crate::clipboard::set_image(width as u32, height as u32, pixels, "image/rgba");
                    self.set_fg(FG_ACCENT);
                    self.print_str("copied image payload\n");
                }
                _ => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: clipimg <w> <h>\n");
                }
            },

            Some("paste") => match crate::clipboard::get_text() {
                Some(text) => {
                    self.set_fg(FG_OUTPUT);
                    self.print_str(&text);
                    self.print_char('\n');
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("paste: clipboard has no text\n");
                }
            },

            Some("exec") => match words.next() {
                Some(path) => {
                    let args: Vec<&str> = words.collect();
                    let abs = resolve_path(&self.cwd, path);
                    match crate::elf::spawn_elf_process_suspended_with_args(&abs, &args) {
                        Ok(pid) => {
                            if self.configure_process_tty(pid, pid) {
                                self.begin_foreground(pid, pid, None, &abs);
                                crate::scheduler::unblock(pid);
                            } else {
                                let _ = crate::scheduler::kill_task(pid, 143);
                            }
                        }
                        Err(err) => {
                            self.set_fg(FG_ERROR);
                            self.print_str("exec: ");
                            self.set_fg(FG_OUTPUT);
                            self.print_str(err.as_str());
                            self.print_char('\n');
                        }
                    }
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: exec <path> [args...]\n");
                }
            },

            Some("sh") | Some("shell") => {
                let abs = "/bin/sh";
                match crate::elf::spawn_elf_process_suspended_with_args(abs, &[]) {
                    Ok(pid) => {
                        if self.configure_process_tty(pid, pid) {
                            self.begin_foreground(pid, pid, None, abs);
                            crate::scheduler::unblock(pid);
                        } else {
                            let _ = crate::scheduler::kill_task(pid, 143);
                        }
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("sh: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err.as_str());
                        self.print_char('\n');
                    }
                }
            }

            Some("browser") | Some("web") | Some("www") => {
                let target = words.next().unwrap_or("browser://home");
                self.pending_browser_url = Some(String::from(target));
                self.set_fg(FG_ACCENT);
                self.print_str("browser: opening ");
                self.set_fg(FG_OUTPUT);
                self.print_str(target);
                self.print_char('\n');
            }

            Some("ipc") => match crate::vfs::vfs_pipe() {
                Some((read_fd, write_fd)) => {
                    let r =
                        crate::elf::spawn_elf_process_with_fds("/bin/piperd", &[], &[(read_fd, 3)]);
                    let w = crate::elf::spawn_elf_process_with_fds(
                        "/bin/pipewr",
                        &[],
                        &[(write_fd, 3)],
                    );
                    crate::vfs::vfs_close(read_fd);
                    crate::vfs::vfs_close(write_fd);
                    match (r, w) {
                        (Ok(_), Ok(_)) => {
                            self.set_fg(FG_ACCENT);
                            self.print_str("pipe demo spawned\n");
                        }
                        _ => {
                            self.set_fg(FG_ERROR);
                            self.print_str("ipc: spawn failed\n");
                        }
                    }
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("ipc: no pipe slots\n");
                }
            },

            Some("keydemo") => match crate::vfs::vfs_pipe() {
                Some((read_fd, write_fd)) => {
                    match crate::elf::spawn_elf_process_with_fds(
                        "/bin/keyecho",
                        &[],
                        &[(read_fd, 3)],
                    ) {
                        Ok(_) => {
                            crate::vfs::vfs_close(read_fd);
                            self.pending_key_sink_fd = Some(write_fd);
                            self.set_fg(FG_ACCENT);
                            self.print_str("keydemo active — ~ ends\n");
                        }
                        Err(err) => {
                            crate::vfs::vfs_close(read_fd);
                            crate::vfs::vfs_close(write_fd);
                            self.set_fg(FG_ERROR);
                            self.print_str("keydemo: ");
                            self.set_fg(FG_OUTPUT);
                            self.print_str(err.as_str());
                            self.print_char('\n');
                        }
                    }
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("keydemo: no pipe slots\n");
                }
            },

            Some("term") => match crate::vfs::vfs_pipe() {
                Some((read_fd, write_fd)) => {
                    match crate::elf::spawn_elf_process_with_stdin("/bin/terminal", &[], read_fd) {
                        Ok(()) => {
                            self.pending_key_sink_fd = Some(write_fd);
                            self.set_fg(FG_ACCENT);
                            self.print_str("userspace terminal — Ctrl+D ends\n");
                        }
                        Err(err) => {
                            crate::vfs::vfs_close(read_fd);
                            crate::vfs::vfs_close(write_fd);
                            self.set_fg(FG_ERROR);
                            self.print_str("term: ");
                            self.set_fg(FG_OUTPUT);
                            self.print_str(err.as_str());
                            self.print_char('\n');
                        }
                    }
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("term: no pipe slots\n");
                }
            },

            Some("usb") => {
                let lines = crate::usb::status_lines();
                if lines.is_empty() {
                    self.set_fg(FG_WARN);
                    self.print_str("USB: no probe data\n");
                } else {
                    self.set_fg(FG_ACCENT);
                    self.print_str("USB STATUS\n");
                    for line in lines {
                        self.set_fg(FG_OUTPUT);
                        self.print_str(&line);
                        self.print_char('\n');
                    }
                }
            }

            Some(unknown) => {
                self.set_fg(FG_ERROR);
                self.print_str(unknown);
                self.set_fg(FG_DIM);
                self.print_str(": not found. ");
                self.set_fg(FG_OUTPUT);
                self.print_str("type help\n");
            }

            None => {}
        }
        if self.foreground_job.is_none() {
            self.print_prompt();
        }
    }

    fn cmd_help(&mut self) {
        let cmds: &[(&str, &str)] = &[
            ("help", "list commands"),
            ("clear", "clear terminal"),
            ("reboot", "restart OS"),
            ("pwd", "print working directory"),
            ("cd <dir>", "change directory"),
            ("ls [path]", "list directory contents"),
            ("touch <path>", "create empty file"),
            ("mkdir <path>", "create folder"),
            ("cat <path>", "print file to terminal"),
            ("hash <path>", "print file length and byte sum"),
            ("write <path> <text>", "write text file"),
            ("rm <path>", "remove file or empty folder"),
            ("ps", "list running processes"),
            ("kill <pid>", "terminate a task"),
            ("wait <pid>", "reap an exited child"),
            ("reap", "reap all exited tasks"),
            ("exec <path>", "run ELF binary"),
            ("sh", "start userspace shell"),
            ("browser [url]", "open URL in Web Browser"),
            ("info", "CPU, memory, and system info"),
            ("uptime", "time since boot"),
            ("usb", "USB controller status"),
            ("devices", "PCI/USB/device registry"),
            ("drivers", "driver binding + /DEV nodes"),
            ("net", "network stack status"),
            ("netproto", "ARP/IP/UDP/DNS/HTTP status"),
            ("tlscheck", "TLS hostname negative checks"),
            ("netapi", "network/settings API toggles"),
            ("dns <host>", "resolve host with staged DNS"),
            ("ping <host>", "send ICMP echo request"),
            ("http|https <host-or-url> [path]", "run kernel web client"),
            ("power <op>", "ACPI power status"),
            ("log", "kernel log tail"),
            ("logs", "open combined log summary"),
            (
                "diagnostics",
                "combined logs/profiler/update/fs/memory status",
            ),
            ("engine [op]", "browser engine port ABI and WPE readiness"),
            ("sysreport [write]", "combined diagnostics report"),
            ("devkit", "SDK docs and app templates"),
            ("profiler", "boot/service/task timing"),
            ("boot <op>", "boot health and last-known-good state"),
            (
                "compositor",
                "FPS, pacing, budget, damage, and cursor telemetry",
            ),
            ("smoothness", "compositor pacing and latency telemetry"),
            ("heap", "heap diagnostics"),
            ("memory", "heap pressure, reclaim, OOM, and per-task memory"),
            ("slab", "slab allocator diagnostics"),
            ("waitq", "kernel wait queue diagnostics"),
            ("writeback", "async disk writeback state"),
            ("selftest", "boot kernel unit-style checks"),
            ("font", "font renderer diagnostics"),
            ("deferred", "deferred work queue"),
            ("tasksnap", "persistent task snapshot"),
            ("fsck", "filesystem check summary"),
            ("coolfs", "CoolFS mount status"),
            ("fsrepair", "repair standard FS dirs"),
            ("recovery [op]", "boot and first-boot recovery"),
            ("update <op>", "verify keys/stage/apply trusted updates"),
            ("mounts", "mount/cache/journal status"),
            ("vfs", "mount table and fd tables"),
            ("path <path>", "inspect normalized VFS path"),
            ("perm <path>", "show owner and mode"),
            ("chmod <mode> <path>", "change CoolFS mode"),
            ("chown <uid>[:gid] <path>", "change CoolFS owner"),
            ("journal", "filesystem journal tail"),
            ("flush", "flush filesystem journal"),
            ("df", "filesystem free space"),
            ("shortcuts", "configured shortcut keys"),
            ("icons", "desktop icon positions"),
            ("access [key on/off]", "accessibility settings"),
            ("apps", "app lifecycle metadata"),
            ("appcats", "app categories"),
            ("pinned [apps...]", "view/set pinned apps"),
            ("pin <item>", "add pinned Start item"),
            ("unpin <item>", "remove pinned Start item"),
            ("startmenu [compact|full]", "view/set Start menu layout"),
            ("recent", "recent apps, files, commands, searches"),
            ("startup [apps...]", "view/set startup apps"),
            ("search <query>", "search indexed files"),
            ("index", "rebuild desktop search index"),
            ("whoami", "current user and task grants"),
            ("id [user]", "user identity and home"),
            ("groups [user]", "group membership"),
            ("login <user> <pass>", "switch active session"),
            ("lock", "lock the desktop session"),
            ("logout", "return to guest session"),
            ("passwd <old> <new>", "change current password"),
            ("setup <user> <pass>", "complete first-run admin setup"),
            (
                "install [status|reset|repair|disks|disk <dev>|verify <dev>]",
                "first-boot and disk installer state",
            ),
            ("account <op>", "admin user management"),
            ("umask [mode]", "view/set file creation mask"),
            ("users", "user/security status"),
            ("pkg <op>", "package payload trust/install/repair/run"),
            ("proc", "process groups and signals"),
            ("zombies", "zombie cleanup policy"),
            ("signal <pid|-pgid> <sig>", "deliver signal to task/group"),
            ("pgroup <pid> [grp]", "view/set process group"),
            ("tty", "current terminal session state"),
            ("events", "event bus tail"),
            ("jobs", "background job history"),
            ("job run|cancel|pause|resume", "manage background jobs"),
            ("fg [id|last]", "resume job in foreground"),
            ("bg [id|last]", "resume job in background"),
            ("services <op>", "durable service supervisor"),
            ("crash", "crash dump summary"),
            ("abi", "kernel/user ABI version"),
            ("notify <op>", "notification history/actions"),
            ("screenshot [path]", "save focused window PPM"),
            ("clip [text]", "shared clipboard"),
            ("clipmimes", "clipboard MIME negotiation"),
            ("clipimg <w> <h>", "copy RGBA image payload"),
            ("paste", "paste shared clipboard text"),
            ("echo <text>", "print text"),
            ("ipc", "pipe demo"),
            ("keydemo", "keyboard event stream"),
            ("term", "userspace terminal"),
        ];
        self.set_fg(FG_ACCENT);
        self.print_str("Commands:\n");
        for &(name, desc) in cmds {
            self.set_fg(FG_PROMPT);
            self.print_str("  ");
            self.print_str(name);
            // pad to column 18
            let name_len = name.len() + 2;
            for _ in name_len..20 {
                self.print_char(' ');
            }
            self.set_fg(FG_DIM);
            self.print_str(desc);
            self.print_char('\n');
        }
    }

    fn cmd_ls(&mut self, path: &str) {
        match crate::vfs::vfs_list_dir(path) {
            Some(mut entries) => {
                entries.sort_by(|a, b| {
                    if a.is_dir == b.is_dir {
                        a.name.cmp(&b.name)
                    } else if a.is_dir {
                        core::cmp::Ordering::Less
                    } else {
                        core::cmp::Ordering::Greater
                    }
                });
                if entries.is_empty() {
                    self.set_fg(FG_DIM);
                    self.print_str("(empty)\n");
                } else {
                    for e in &entries {
                        if e.is_dir {
                            self.set_fg(FG_DIR);
                            self.print_str(&e.name);
                            self.print_char('/');
                        } else {
                            self.set_fg(FG_OUTPUT);
                            self.print_str(&e.name);
                        }
                        self.print_char('\n');
                    }
                }
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("ls: no such directory\n");
            }
        }
    }

    fn cmd_cat(&mut self, path: &str) {
        match crate::vfs::vfs_read_file(path) {
            Some(bytes) => match core::str::from_utf8(&bytes) {
                Ok(text) => {
                    self.set_fg(FG_OUTPUT);
                    self.print_str(text);
                    if !text.ends_with('\n') {
                        self.print_char('\n');
                    }
                }
                Err(_) => {
                    self.set_fg(FG_WARN);
                    self.print_str("cat: binary file (");
                    self.print_u64(bytes.len() as u64);
                    self.print_str(" bytes)\n");
                }
            },
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("cat: file not found\n");
            }
        }
    }

    fn cmd_hash(&mut self, path: &str) {
        match crate::vfs::vfs_read_file(path) {
            Some(bytes) => {
                let sum = bytes
                    .iter()
                    .fold(0u64, |acc, byte| acc.wrapping_add(*byte as u64));
                self.set_fg(FG_OUTPUT);
                self.print_str("hash ");
                self.print_str(path);
                self.print_str(" len=");
                self.print_u64(bytes.len() as u64);
                self.print_str(" sum=");
                self.print_u64(sum);
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("hash: file not found\n");
            }
        }
    }

    fn cmd_perm(&mut self, path: &str) {
        match crate::vfs::vfs_metadata(path) {
            Some(meta) => {
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_str(" ");
                self.print_str(if meta.is_dir { "dir" } else { "file" });
                self.print_str(" uid=");
                self.print_u64(meta.uid as u64);
                self.print_str(" gid=");
                self.print_u64(meta.gid as u64);
                self.print_str(" mode=");
                self.print_str(&crate::security::format_mode(meta.mode));
                self.print_str(" size=");
                self.print_u64(meta.size);
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("perm: not found or denied\n");
            }
        }
    }

    fn cmd_chmod(&mut self, mode: &str, path: &str) {
        let Some(mode) = crate::security::parse_mode(mode) else {
            self.set_fg(FG_ERROR);
            self.print_str("chmod: invalid mode\n");
            return;
        };
        match crate::vfs::vfs_chmod(path, mode) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("chmod ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("chmod: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_chown(&mut self, owner: &str, path: &str) {
        let Some((uid, gid)) = parse_owner(owner) else {
            self.set_fg(FG_ERROR);
            self.print_str("chown: invalid owner\n");
            return;
        };
        match crate::vfs::vfs_chown(path, uid, gid) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("chown ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("chown: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_write_file(&mut self, path: &str, text: &str) {
        match crate::vfs::vfs_create_file(path) {
            Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("write: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
                return;
            }
        }
        match crate::vfs::vfs_write_file(path, text.as_bytes()) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("wrote ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("write: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_rm(&mut self, path: &str) {
        match crate::vfs::vfs_delete(path) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("removed ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("rm: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_touch(&mut self, path: &str) {
        match crate::vfs::vfs_create_file(path) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("created ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("touch: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_mkdir(&mut self, path: &str) {
        match crate::vfs::vfs_create_dir(path) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("created ");
                self.set_fg(FG_DIR);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("mkdir: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_ps(&mut self) {
        // Copy task info while holding the lock, then drop it before printing.
        let tasks: Vec<(
            usize,
            &'static str,
            crate::scheduler::TaskStatus,
            bool,
            bool,
            Option<u64>,
            Option<usize>,
            u32,
        )> = {
            let sched = crate::scheduler::SCHEDULER.lock();
            let cur = sched.current;
            sched
                .tasks
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    (
                        i,
                        t.name,
                        t.status,
                        i == cur,
                        t.pml4.is_some(),
                        t.exit_code,
                        t.parent,
                        t.credentials.uid,
                    )
                })
                .collect()
        };

        self.set_fg(FG_ACCENT);
        self.print_str("PID  PPID  UID   RING  STATUS   EXIT  NAME\n");
        self.set_fg(FG_DIM);
        self.print_str("---  ----  ----  ----  -------  ----  ----\n");
        for (id, name, status, is_cur, is_user, exit_code, parent, uid) in tasks {
            self.set_fg(if is_cur { FG_PROMPT } else { FG_OUTPUT });
            self.print_u64(id as u64);
            self.print_str("    ");
            self.set_fg(FG_DIM);
            if let Some(parent) = parent {
                self.print_u64(parent as u64);
            } else {
                self.print_char('-');
            }
            self.print_str("     ");
            self.set_fg(FG_DIM);
            self.print_u64(uid as u64);
            self.print_str("  ");
            self.set_fg(FG_DIM);
            let ring = if is_user { "u" } else { "k" };
            self.print_str(ring);
            self.print_str("     ");
            self.set_fg(FG_OUTPUT);
            let status_str = match status {
                crate::scheduler::TaskStatus::Running => "running",
                crate::scheduler::TaskStatus::Ready => "ready  ",
                crate::scheduler::TaskStatus::Blocked => "blocked",
                crate::scheduler::TaskStatus::Stopped => "stopped",
                crate::scheduler::TaskStatus::Exited => "exited ",
                crate::scheduler::TaskStatus::Reaped => "reaped ",
            };
            self.print_str(status_str);
            self.print_str("  ");
            self.set_fg(FG_DIM);
            if let Some(code) = exit_code {
                self.print_u64(code);
            } else {
                self.print_char('-');
            }
            self.print_str("     ");
            if is_cur {
                self.set_fg(FG_PROMPT);
            } else if status == crate::scheduler::TaskStatus::Exited {
                self.set_fg(FG_DIM);
            } else {
                self.set_fg(FG_OUTPUT);
            }
            self.print_str(name);
            self.print_char('\n');
        }
    }

    fn cmd_kill(&mut self, pid: usize) {
        match crate::scheduler::kill_task(pid, 130) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("killed task ");
                self.set_fg(FG_OUTPUT);
                self.print_u64(pid as u64);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("kill: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_wait(&mut self, pid: usize) {
        match crate::scheduler::waitpid(0, pid) {
            Ok(code) => {
                self.set_fg(FG_ACCENT);
                self.print_str("reaped ");
                self.set_fg(FG_OUTPUT);
                self.print_u64(pid as u64);
                self.print_str(" exit ");
                self.print_u64(code);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("wait: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_reap(&mut self) {
        let count = crate::scheduler::reap_all_exited(0);
        self.set_fg(FG_ACCENT);
        self.print_str("reaped ");
        self.set_fg(FG_OUTPUT);
        self.print_u64(count as u64);
        self.print_str(" task(s)\n");
    }

    fn cmd_devices(&mut self) {
        crate::device_registry::refresh_pci();
        self.cmd_lines("DEVICES", crate::device_registry::lines());
    }

    fn cmd_sysreport(&mut self, op: Option<&str>) {
        match op {
            Some("write") => match crate::sysreport::write_report() {
                Ok(()) => {
                    self.set_fg(FG_ACCENT);
                    self.print_str("wrote ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(crate::sysreport::report_path());
                    self.print_char('\n');
                }
                Err(err) => {
                    self.set_fg(FG_ERROR);
                    self.print_str("sysreport: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(err);
                    self.print_char('\n');
                }
            },
            _ => self.cmd_lines("SYSREPORT", crate::sysreport::lines()),
        }
    }

    fn cmd_boot(&mut self, args: Vec<&str>) {
        match args.as_slice() {
            [] | ["status"] => self.cmd_lines("BOOT HEALTH", crate::boot_health::status_lines()),
            ["history"] | ["log"] => {
                self.cmd_lines("BOOT HISTORY", crate::boot_health::history_lines())
            }
            ["mark-good"] => {
                if !self.require_admin("boot") {
                    return;
                }
                crate::boot_health::mark_good("manual mark-good");
                self.set_fg(FG_ACCENT);
                self.print_str("boot: marked good\n");
            }
            ["fail-validation", update_id, rest @ ..] => {
                if !self.require_admin("boot") {
                    return;
                }
                let reason = collect_words(rest.iter().copied());
                match crate::boot_health::record_failed_validation(update_id, &reason) {
                    Ok(()) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("boot: validation failure recorded\n");
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("boot: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str(
                    "usage: boot [status|history|mark-good|fail-validation <update-id> <reason>]\n",
                );
            }
        }
    }

    fn cmd_devkit(&mut self) {
        self.cmd_lines(
            "DEVKIT",
            alloc::vec![
                alloc::format!("coolOS devkit ABI={}", crate::abi::version()),
                String::from("docs=/SDK/README.TXT"),
                String::from("app_template=/SDK/APP_TEMPLATE.RS"),
                String::from("package_template=/SDK/PACKAGE_TEMPLATE.PKG"),
                String::from("browser_engine_port=/SDK/BROWSER_ENGINE_PORT.TXT"),
                String::from("example: exec /bin/devkit"),
            ],
        );
    }

    fn cmd_lines(&mut self, title: &str, lines: Vec<String>) {
        self.set_fg(FG_ACCENT);
        self.print_str(title);
        self.print_char('\n');
        if lines.is_empty() {
            self.set_fg(FG_DIM);
            self.print_str("(none)\n");
            return;
        }
        for line in lines {
            self.set_fg(FG_OUTPUT);
            self.print_str(&line);
            self.print_char('\n');
        }
    }

    fn cmd_recovery(&mut self, args: Vec<&str>) {
        match args.as_slice() {
            ["repair"] => self.cmd_lines("RECOVERY REPAIR", crate::recovery::repair_lines()),
            ["firstboot"] | ["firstboot", "status"] => {
                self.cmd_lines("RECOVERY FIRSTBOOT", crate::recovery::firstboot_status_lines())
            }
            ["firstboot", "reset"] => {
                self.cmd_lines("RECOVERY FIRSTBOOT", crate::recovery::firstboot_reset_lines())
            }
            ["firstboot", "repair"] => {
                self.cmd_lines("RECOVERY FIRSTBOOT", crate::recovery::firstboot_repair_lines())
            }
            ["firstboot", ..] => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: recovery firstboot [status|reset|repair]\n");
            }
            ["install", "disks"] => self.cmd_lines("RECOVERY INSTALL", crate::installer::disks_lines()),
            ["install", "disk", target] => self.cmd_lines(
                "RECOVERY INSTALL",
                crate::installer::install_to_device_name(target),
            ),
            ["install", "verify", target] => self.cmd_lines(
                "RECOVERY INSTALL",
                crate::installer::verify_device_name(target),
            ),
            ["install", ..] => {
                self.set_fg(FG_ERROR);
                self.print_str(
                    "usage: recovery install [disks|disk <ide-device>|verify <ide-device>]\n",
                );
            }
            ["rollback"] => {
                if !self.require_admin("recovery") {
                    return;
                }
                match crate::updates::rollback() {
                    Ok(()) => {
                        crate::boot_health::mark_manual_rollback("recovery rollback");
                        let mut lines = alloc::vec![String::from("update rollback ok")];
                        lines.extend(crate::updates::status_lines());
                        lines.extend(crate::boot_health::status_lines());
                        self.cmd_lines("RECOVERY ROLLBACK", lines);
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("recovery rollback: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            ["fsck-on-boot", "on"] | ["on"] => {
                self.cmd_lines("RECOVERY", crate::recovery::set_fsck_on_boot(true))
            }
            ["fsck-on-boot", "off"] | ["off"] => {
                self.cmd_lines("RECOVERY", crate::recovery::set_fsck_on_boot(false))
            }
            ["fsck-on-boot"] => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: recovery fsck-on-boot <on|off>\n");
            }
            [other, ..] => {
                self.set_fg(FG_ERROR);
                self.print_str("recovery: unknown op ");
                self.set_fg(FG_OUTPUT);
                self.print_str(other);
                self.print_char('\n');
            }
            [] => self.cmd_lines("RECOVERY", crate::recovery::status_lines()),
        }
    }

    fn cmd_update(&mut self, args: Vec<&str>) {
        match args.as_slice() {
            [] | ["status"] => self.cmd_lines("UPDATE STATUS", crate::updates::status_lines()),
            ["verify"] => self.cmd_lines("UPDATE VERIFY", crate::updates::verify_lines()),
            ["keys"] => self.cmd_lines("UPDATE TRUST KEYS", crate::updates::trust_key_lines()),
            ["history"] | ["log"] => {
                self.cmd_lines("UPDATE HISTORY", crate::updates::history_lines())
            }
            ["sign"] => {
                if !self.require_admin("update") {
                    return;
                }
                match crate::updates::sign_staged() {
                    Ok(()) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("update: signed\n");
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("update: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            ["sign-as", key] => {
                if !self.require_admin("update") {
                    return;
                }
                match crate::updates::sign_staged_as(key) {
                    Ok(()) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("update: signed as ");
                        self.print_str(key);
                        self.print_char('\n');
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("update: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            ["corrupt-payload", rest @ ..] => {
                if !self.require_admin("update") {
                    return;
                }
                let text = collect_words(rest.iter().copied());
                match crate::updates::corrupt_staged_payload(&text) {
                    Ok(()) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("update: payload corrupted\n");
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("update: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            ["unsign"] => {
                if !self.require_admin("update") {
                    return;
                }
                match crate::updates::remove_staged_signature() {
                    Ok(()) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("update: unsigned\n");
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("update: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            ["stage", target, rest @ ..] => {
                if !self.require_admin("update") {
                    return;
                }
                let target = resolve_path(&self.cwd, target);
                let text = collect_words(rest.iter().copied());
                match crate::updates::stage_text(&target, &text) {
                    Ok(()) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("update: staged\n");
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("update: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            ["stage-version", target, version, rest @ ..] => {
                if !self.require_admin("update") {
                    return;
                }
                let Some(version) = parse_u64(version) else {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: update stage-version <path> <version> <text>\n");
                    return;
                };
                let target = resolve_path(&self.cwd, target);
                let text = collect_words(rest.iter().copied());
                match crate::updates::stage_text_with_version(&target, version, &text) {
                    Ok(()) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("update: staged\n");
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("update: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            ["apply"] => {
                if !self.require_admin("update") {
                    return;
                }
                match crate::updates::apply() {
                    Ok(()) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("update: applied\n");
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("update: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            ["rollback"] => {
                if !self.require_admin("update") {
                    return;
                }
                match crate::updates::rollback() {
                    Ok(()) => {
                        crate::boot_health::mark_manual_rollback("manual update rollback");
                        self.set_fg(FG_ACCENT);
                        self.print_str("update: rollback ok\n");
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("update: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str(
                    "usage: update [status|verify|keys|history|sign|sign-as <key>|stage <path> <text>|stage-version <path> <version> <text>|apply|rollback]\n",
                );
            }
        }
    }

    fn cmd_whoami(&mut self) {
        let user = crate::security::current_user();
        let creds = crate::security::current_credentials();
        self.set_fg(FG_OUTPUT);
        self.print_str(&user.name);
        self.print_str(" uid=");
        self.print_u64(creds.uid as u64);
        self.print_str(" gid=");
        self.print_u64(creds.gid as u64);
        self.print_str(" caps=");
        self.print_str(&crate::security::capability_label(creds.caps));
        self.print_str(" home=");
        self.print_str(&user.home);
        self.print_char('\n');
    }

    fn cmd_id(&mut self, user: Option<&str>) {
        let user = match user {
            Some(name) => crate::security::user_by_name(name),
            None => Some(crate::security::current_user()),
        };
        match user {
            Some(user) => {
                self.set_fg(FG_OUTPUT);
                self.print_str(&user.name);
                self.print_str(" uid=");
                self.print_u64(user.uid as u64);
                self.print_str(" gid=");
                self.print_u64(user.gid as u64);
                self.print_str(" role=");
                self.print_str(&user.role);
                self.print_str(" home=");
                self.print_str(&user.home);
                self.print_str(" login=");
                self.print_str(if user.login_enabled {
                    "enabled"
                } else {
                    "disabled"
                });
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("id: no such user\n");
            }
        }
    }

    fn cmd_groups(&mut self, user: Option<&str>) {
        let name = user
            .map(String::from)
            .unwrap_or_else(|| crate::security::current_user().name);
        match crate::security::groups_for(&name) {
            Some(groups) => {
                self.set_fg(FG_OUTPUT);
                self.print_str(&name);
                self.print_str(":");
                for group in groups {
                    self.print_char(' ');
                    self.print_str(&group.name);
                    self.print_char('(');
                    self.print_u64(group.gid as u64);
                    self.print_char(')');
                }
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("groups: no such user\n");
            }
        }
    }

    fn cmd_login(&mut self, user: Option<&str>, password: Option<&str>) {
        let (Some(user), Some(password)) = (user, password) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: login <user> <password>\n");
            return;
        };
        match crate::security::login(user, password) {
            Ok(user) => {
                self.set_fg(FG_ACCENT);
                self.print_str("session user ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&user.name);
                self.print_str(" uid=");
                self.print_u64(user.uid as u64);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("login: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_logout(&mut self) {
        let user = crate::security::logout();
        crate::wm::request_session_lock();
        self.set_fg(FG_ACCENT);
        self.print_str("session user ");
        self.set_fg(FG_OUTPUT);
        self.print_str(&user.name);
        self.print_str(" uid=");
        self.print_u64(user.uid as u64);
        self.print_char('\n');
    }

    fn cmd_lock(&mut self) {
        crate::wm::request_session_lock();
        self.set_fg(FG_ACCENT);
        self.print_str("session locked\n");
    }

    fn cmd_passwd(&mut self, old_password: Option<&str>, new_password: Option<&str>) {
        let (Some(old_password), Some(new_password)) = (old_password, new_password) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: passwd <old-password> <new-password>\n");
            return;
        };
        match crate::security::change_password(old_password, new_password) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("password updated\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("passwd: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_setup(&mut self, user: Option<&str>, password: Option<&str>) {
        let (Some(user), Some(password)) = (user, password) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: setup <admin-user> <password>\n");
            return;
        };
        match crate::security::complete_first_run_admin(user, password) {
            Ok(user) => {
                self.set_fg(FG_ACCENT);
                self.print_str("first-run admin ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&user.name);
                self.print_str(" uid=");
                self.print_u64(user.uid as u64);
                self.print_char('\n');
            }
            Err(err) => self.print_account_error("setup", err),
        }
    }

    fn cmd_install(&mut self, args: Vec<&str>) {
        match args.first().copied().unwrap_or("status") {
            "status" => self.cmd_lines("INSTALL", crate::security::first_boot_status_lines()),
            "disks" => self.cmd_lines("INSTALL DISKS", crate::installer::disks_lines()),
            "disk" => {
                let Some(target) = args.get(1).copied() else {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: install disk <ide-device>\n");
                    return;
                };
                if !self.require_install_mutation("install") {
                    return;
                }
                self.cmd_lines(
                    "INSTALL DISK",
                    crate::installer::install_to_device_name(target),
                );
            }
            "verify" => {
                let Some(target) = args.get(1).copied() else {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: install verify <ide-device>\n");
                    return;
                };
                if !self.require_install_mutation("install") {
                    return;
                }
                self.cmd_lines(
                    "INSTALL VERIFY",
                    crate::installer::verify_device_name(target),
                );
            }
            "reset" => {
                if !self.require_admin("install") {
                    return;
                }
                match crate::security::reset_first_boot_admin() {
                    Ok(lines) => self.cmd_lines("INSTALL RESET", lines),
                    Err(err) => self.print_account_error("install reset", err),
                }
            }
            "repair" => {
                if !self.require_admin("install") {
                    return;
                }
                match crate::security::repair_first_boot_admin() {
                    Ok(lines) => self.cmd_lines("INSTALL REPAIR", lines),
                    Err(err) => self.print_account_error("install repair", err),
                }
            }
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: install [status|reset|repair|disks|disk <device>|verify <device>]\n");
            }
        }
    }

    fn cmd_account(&mut self, args: Vec<&str>) {
        let Some(op) = args.first().copied() else {
            self.print_account_usage();
            return;
        };
        match op {
            "list" | "ls" => self.cmd_lines("ACCOUNTS", crate::security::lines()),
            "add" => {
                let (Some(name), Some(password)) = (args.get(1), args.get(2)) else {
                    self.print_account_usage();
                    return;
                };
                let role = args.get(3).copied().unwrap_or("user");
                match crate::security::create_user(name, password, role) {
                    Ok(user) => self.print_account_user("added", &user),
                    Err(err) => self.print_account_error("account add", err),
                }
            }
            "enable" => {
                let Some(name) = args.get(1) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::set_user_enabled(name, true) {
                    Ok(user) => self.print_account_user("enabled", &user),
                    Err(err) => self.print_account_error("account enable", err),
                }
            }
            "disable" => {
                let Some(name) = args.get(1) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::set_user_enabled(name, false) {
                    Ok(user) => self.print_account_user("disabled", &user),
                    Err(err) => self.print_account_error("account disable", err),
                }
            }
            "role" => {
                let (Some(name), Some(role)) = (args.get(1), args.get(2)) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::set_user_role(name, role) {
                    Ok(user) => self.print_account_user("role", &user),
                    Err(err) => self.print_account_error("account role", err),
                }
            }
            "pass" | "password" => {
                let (Some(name), Some(password)) = (args.get(1), args.get(2)) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::reset_user_password(name, password) {
                    Ok(user) => self.print_account_user("password", &user),
                    Err(err) => self.print_account_error("account pass", err),
                }
            }
            "delete" | "del" | "remove" | "rm" => {
                let Some(name) = args.get(1) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::delete_user(name) {
                    Ok(user) => self.print_account_user("deleted", &user),
                    Err(err) => self.print_account_error("account delete", err),
                }
            }
            _ => self.print_account_usage(),
        }
    }

    fn print_account_usage(&mut self) {
        self.set_fg(FG_ERROR);
        self.print_str("usage: account list|add <user> <pass> [admin|user]|enable <user>|disable <user>|role <user> <admin|user>|pass <user> <pass>|delete <user>\n");
    }

    fn print_account_user(&mut self, action: &str, user: &crate::security::User) {
        self.set_fg(FG_ACCENT);
        self.print_str("account ");
        self.print_str(action);
        self.print_char(' ');
        self.set_fg(FG_OUTPUT);
        self.print_str(&user.name);
        self.print_str(" uid=");
        self.print_u64(user.uid as u64);
        self.print_str(" role=");
        self.print_str(&user.role);
        self.print_str(" login=");
        self.print_str(if user.login_enabled {
            "enabled"
        } else {
            "disabled"
        });
        self.print_char('\n');
    }

    fn print_account_error(&mut self, label: &str, err: crate::security::AccountError) {
        self.set_fg(FG_ERROR);
        self.print_str(label);
        self.print_str(": ");
        self.set_fg(FG_OUTPUT);
        self.print_str(err.as_str());
        self.print_char('\n');
    }

    fn cmd_umask(&mut self, mode: Option<&str>) {
        match mode {
            Some(mode) => {
                let Some(mode) = crate::security::parse_mode(mode) else {
                    self.set_fg(FG_ERROR);
                    self.print_str("umask: invalid mode\n");
                    return;
                };
                let old = crate::security::set_umask(mode);
                self.set_fg(FG_ACCENT);
                self.print_str("umask ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&crate::security::format_mode(old));
                self.print_str(" -> ");
                self.print_str(&crate::security::format_mode(mode));
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_OUTPUT);
                self.print_str("umask ");
                self.print_str(&crate::security::format_mode(crate::security::umask()));
                self.print_char('\n');
            }
        }
    }

    fn cmd_http(&mut self, scheme: &str, host: &str, path: &str) {
        let result = if host.starts_with("http://") || host.starts_with("https://") {
            crate::net::web_get_response(host)
        } else if scheme == "https" {
            let mut url = String::from("https://");
            url.push_str(host);
            if path.starts_with('/') {
                url.push_str(path);
            } else {
                url.push('/');
                url.push_str(path);
            }
            crate::net::web_get_response(&url)
        } else {
            crate::net::http_get_response(host, path)
        };
        match result {
            Ok(response) => {
                self.set_fg(FG_ACCENT);
                self.print_str("HTTP CLIENT\n");
                self.set_fg(FG_OUTPUT);
                self.print_str(&response.status_line);
                self.print_char('\n');
                self.print_str("resolved ");
                self.print_str(&response.host);
                self.print_str(&response.path);
                self.print_str(" -> ");
                self.print_str(&crate::net::ipv4_string(response.resolved_addr));
                self.print_char('\n');
                if let Some(root) = response.tls_trust_root {
                    self.print_str("tls root ");
                    self.print_str(root);
                    self.print_char('\n');
                }
                if response.redirect_count > 0 {
                    self.print_str("final ");
                    self.print_str(&response.final_url);
                    self.print_str(" redirects=");
                    self.print_u64(response.redirect_count as u64);
                    self.print_char('\n');
                }
                self.print_str(&response.request);
                self.print_str(&response.body);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("http: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err);
                self.print_char('\n');
            }
        }
    }

    fn cmd_access(&mut self, key: Option<&str>, value: Option<&str>) {
        match (key, value.and_then(parse_bool_word)) {
            (Some(key), Some(value)) => {
                if crate::accessibility::set(key, value) {
                    self.set_fg(FG_ACCENT);
                    self.print_str("updated accessibility setting\n");
                } else {
                    self.set_fg(FG_ERROR);
                    self.print_str("access: unknown key\n");
                }
            }
            (None, _) => self.cmd_lines("ACCESSIBILITY", crate::accessibility::lines()),
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str(
                    "usage: access <keyboard_nav|focus_rings|large_text|reduced_motion> <on|off>\n",
                );
            }
        }
    }

    fn cmd_recent(&mut self) {
        let mut lines = Vec::new();
        lines.push(String::from("apps:"));
        lines.extend(crate::app_lifecycle::recent_apps());
        lines.push(String::from("files:"));
        lines.extend(crate::app_lifecycle::recent_files());
        lines.push(String::from("commands:"));
        lines.extend(crate::app_lifecycle::recent_commands());
        lines.push(String::from("searches:"));
        lines.extend(crate::app_lifecycle::recent_searches());
        self.cmd_lines("RECENT", lines);
    }

    fn cmd_startmenu(&mut self, mode: Option<&str>) {
        match mode {
            Some("compact") => {
                crate::app_lifecycle::set_start_menu_compact(true);
                self.print_str("Start menu compact layout enabled\n");
            }
            Some("full") => {
                crate::app_lifecycle::set_start_menu_compact(false);
                self.print_str("Start menu full layout enabled\n");
            }
            Some(_) => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: startmenu [compact|full]\n");
            }
            None => self.cmd_lines("START MENU", crate::app_lifecycle::lines()),
        }
    }

    fn cmd_pinned(&mut self, apps: Vec<&str>) {
        if apps.is_empty() {
            self.cmd_lines("PINNED APPS", crate::app_lifecycle::pinned_apps());
            return;
        }
        crate::app_lifecycle::set_pinned(apps.iter().map(|app| String::from(*app)).collect());
        self.set_fg(FG_ACCENT);
        self.print_str("pinned apps updated\n");
    }

    fn cmd_pin(&mut self, item: String) {
        if item.is_empty() {
            self.set_fg(FG_ERROR);
            self.print_str("usage: pin <item>\n");
            return;
        }
        if crate::app_lifecycle::is_pinned(&item) {
            self.set_fg(FG_WARN);
            self.print_str("pinned item already exists\n");
            return;
        }
        crate::app_lifecycle::pin_item(&item);
        self.set_fg(FG_ACCENT);
        self.print_str("pinned item added\n");
    }

    fn cmd_unpin(&mut self, item: String) {
        if item.is_empty() {
            self.set_fg(FG_ERROR);
            self.print_str("usage: unpin <item>\n");
            return;
        }
        if crate::app_lifecycle::unpin_item(&item) {
            self.print_str("pinned item removed\n");
        } else {
            self.set_fg(FG_WARN);
            self.print_str("pinned item not found\n");
        }
    }

    fn cmd_startup(&mut self, apps: Vec<&str>) {
        if apps.is_empty() {
            self.cmd_lines("STARTUP APPS", crate::app_lifecycle::startup_apps());
            return;
        }
        crate::app_lifecycle::set_startup(apps.iter().map(|app| String::from(*app)).collect());
        self.set_fg(FG_ACCENT);
        self.print_str("startup apps updated\n");
    }

    fn cmd_pkg(&mut self, op: Option<&str>, arg: Option<&str>, args: Vec<&str>) {
        match (op, arg) {
            (None, _) | (Some("list"), _) => self.cmd_lines("PACKAGES", crate::packages::lines()),
            (Some("keys"), _) => self.cmd_lines("PACKAGE TRUST KEYS", crate::packages::key_lines()),
            (Some("history"), _) => {
                self.cmd_lines("PACKAGE HISTORY", crate::packages::history_lines())
            }
            (Some("transaction"), _) | (Some("txn"), _) => {
                self.cmd_lines("PACKAGE TRANSACTION", crate::packages::transaction_lines())
            }
            (Some("info"), Some(value)) => {
                let value = resolve_path_if_archive(&self.cwd, value);
                self.cmd_lines("PACKAGE INFO", crate::packages::info_lines(&value));
            }
            (Some("verify"), Some(value)) => {
                let value = resolve_path_if_archive(&self.cwd, value);
                self.cmd_lines("PACKAGE VERIFY", crate::packages::verify_lines(&value));
            }
            (Some("install"), Some(id)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                let id = resolve_path_if_archive(&self.cwd, id);
                self.print_result("pkg", crate::packages::install(&id));
            }
            (Some("install-fail"), Some(id)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                let id = resolve_path_if_archive(&self.cwd, id);
                self.print_result("pkg", crate::packages::install_archive_with_fault(&id));
            }
            (Some("remove"), Some(id)) | (Some("uninstall"), Some(id)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                self.print_result("pkg", crate::packages::uninstall(id));
            }
            (Some("repair"), Some(id)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                self.print_result("pkg", crate::packages::repair(id));
            }
            (Some("sign"), Some(path)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                let path = resolve_path(&self.cwd, path);
                self.print_result("pkg", crate::packages::sign_archive(&path));
            }
            (Some("sign-as"), Some(path)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                let Some(key) = args.first() else {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: pkg sign-as <path> <key>\n");
                    return;
                };
                let path = resolve_path(&self.cwd, path);
                self.print_result("pkg", crate::packages::sign_archive_as(&path, key));
            }
            (Some("unsign"), Some(path)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                let path = resolve_path(&self.cwd, path);
                self.print_result("pkg", crate::packages::remove_signature(&path));
            }
            (Some("tamper"), Some(path)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                let path = resolve_path(&self.cwd, path);
                self.print_result("pkg", crate::packages::tamper_archive_name(&path));
            }
            (Some("tamper-payload"), Some(path)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                let path = resolve_path(&self.cwd, path);
                self.print_result("pkg", crate::packages::tamper_archive_payload(&path));
            }
            (Some("deps"), Some(path)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                let path = resolve_path(&self.cwd, path);
                let deps = collect_words(args.iter().copied());
                self.print_result(
                    "pkg",
                    crate::packages::set_archive_dependencies(&path, &deps),
                );
            }
            (Some("break"), Some(id)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                self.print_result("pkg", crate::packages::break_installed(id));
            }
            (Some("break-payload"), Some(id)) | (Some("payload-break"), Some(id)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                self.print_result("pkg", crate::packages::break_installed_payload(id));
            }
            (Some("run"), Some(id)) | (Some("launch"), Some(id)) => {
                match crate::packages::launch(id, &args) {
                    Ok(launch) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("pkg: spawned ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(&launch.exec_path);
                        self.print_str(" pid=");
                        self.print_u64(launch.pid as u64);
                        self.print_char('\n');
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("pkg: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: pkg [list|keys|history|transaction|info <id|path>|verify <id|path>|install <id|path>|remove <id>|repair <id>|run <id> [args...]]\n");
                self.print_str("       pkg [sign <path>|sign-as <path> <key>|unsign <path>|tamper <path>|tamper-payload <path>|deps <path> [ids...]|break <id>|break-payload <id>|install-fail <path>]\n");
            }
        }
    }

    fn cmd_engine(&mut self, op: Option<&str>) {
        match op {
            None | Some("status") => {
                self.cmd_lines("BROWSER ENGINE PORT", crate::browser_engine::status_lines())
            }
            Some("abi") | Some("manifest") => self.cmd_lines(
                "BROWSER ENGINE ABI",
                crate::browser_engine::manifest_lines(),
            ),
            Some("requirements") | Some("reqs") => self.cmd_lines(
                "BROWSER ENGINE REQUIREMENTS",
                crate::browser_engine::requirement_lines(),
            ),
            Some("config") => self.cmd_lines(
                "BROWSER ENGINE CONFIG",
                crate::browser_engine::config_lines(),
            ),
            Some("log") | Some("history") => {
                self.cmd_lines("BROWSER ENGINE LOG", crate::browser_engine::log_lines())
            }
            Some("recovery") | Some("health") => self.cmd_lines(
                "BROWSER ENGINE RECOVERY",
                crate::browser_engine::recovery_lines(),
            ),
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: engine [status|abi|requirements|config|log|recovery]\n");
            }
        }
    }

    fn cmd_signal(&mut self, pid: Option<&str>, signal: Option<&str>) {
        let Some(target) = pid else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: signal <pid|-pgid> <term|int|usr1|stop|cont>\n");
            return;
        };
        let Some(signal) = signal.and_then(crate::process_model::Signal::parse) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: signal <pid|-pgid> <term|int|usr1|stop|cont>\n");
            return;
        };
        if let Some(group_text) = target.strip_prefix('-') {
            let Some(group) = parse_usize(group_text) else {
                self.set_fg(FG_ERROR);
                self.print_str("usage: signal <pid|-pgid> <term|int|usr1|stop|cont>\n");
                return;
            };
            match crate::scheduler::send_signal_to_group(group, signal) {
                Ok(count) => {
                    self.set_fg(FG_ACCENT);
                    self.print_str("signal delivered to ");
                    self.set_fg(FG_OUTPUT);
                    self.print_u64(count as u64);
                    self.print_str(" task(s)\n");
                }
                Err(err) => {
                    self.set_fg(FG_ERROR);
                    self.print_str("signal: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(err.as_str());
                    self.print_char('\n');
                }
            }
            return;
        }
        let Some(pid) = parse_usize(target) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: signal <pid|-pgid> <term|int|usr1|stop|cont>\n");
            return;
        };
        match crate::scheduler::send_signal(pid, signal) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("signal delivered\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("signal: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_pgroup(&mut self, pid: Option<&str>, group: Option<&str>) {
        let Some(pid) = pid.and_then(parse_usize) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: pgroup <pid> [group]\n");
            return;
        };
        if group.is_none() {
            match crate::scheduler::get_process_group(pid) {
                Ok(group) => {
                    self.set_fg(FG_ACCENT);
                    self.print_str("process group ");
                    self.set_fg(FG_OUTPUT);
                    self.print_u64(group as u64);
                    self.print_char('\n');
                }
                Err(err) => {
                    self.set_fg(FG_ERROR);
                    self.print_str("pgroup: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(err.as_str());
                    self.print_char('\n');
                }
            }
            return;
        }
        let Some(group) = group.and_then(parse_usize) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: pgroup <pid> [group]\n");
            return;
        };
        match crate::scheduler::set_process_group(pid, group) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("process group updated\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("pgroup: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_services(&mut self, op: Option<&str>, name: Option<&str>) {
        match (op, name) {
            (None, _) | (Some("list"), _) => self.cmd_lines("SERVICES", crate::services::lines()),
            (Some("history"), _) => {
                self.cmd_lines("SERVICE HISTORY", crate::services::history_lines())
            }
            (Some("recovery"), _) | (Some("health"), _) => {
                self.cmd_lines("SERVICE RECOVERY", crate::services::recovery_lines())
            }
            (Some("run"), _) => {
                if !self.require_admin("services") {
                    return;
                }
                crate::services::supervise_once();
                self.set_fg(FG_ACCENT);
                self.print_str("service supervisor tick\n");
            }
            (Some("start"), Some(name)) => {
                if !self.require_admin("services") {
                    return;
                }
                self.print_bool("service", crate::services::start(name));
            }
            (Some("restart"), Some(name)) => {
                if !self.require_admin("services") {
                    return;
                }
                self.print_bool("service", crate::services::restart(name));
            }
            (Some("stop"), Some(name)) => {
                if !self.require_admin("services") {
                    return;
                }
                self.print_bool("service", crate::services::stop(name));
            }
            (Some("fail"), Some(name)) => {
                if !self.require_admin("services") {
                    return;
                }
                self.print_bool("service", crate::services::fail(name));
            }
            (Some("status"), Some(name)) => match crate::services::status_lines(name) {
                Some(lines) => self.cmd_lines("SERVICE", lines),
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("services: no such service\n");
                }
            },
            (Some(name), None) => match crate::services::status_lines(name) {
                Some(lines) => self.cmd_lines("SERVICE", lines),
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("services: no such service\n");
                }
            },
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: services [list|history|recovery|run|<name>|status <name>|start <name>|restart <name>|stop <name>|fail <name>]\n");
            }
        }
    }

    fn cmd_job(&mut self, args: Vec<&str>) {
        let Some(op) = args.first().copied() else {
            self.set_fg(FG_ERROR);
            self.print_str(
                "usage: job run <path> [args...] | job <cancel|pause|resume> <id|last>\n",
            );
            return;
        };
        if op == "run" {
            let Some(path) = args.get(1).copied() else {
                self.set_fg(FG_ERROR);
                self.print_str("usage: job run <path> [args...]\n");
                return;
            };
            let exec_args: Vec<&str> = args.iter().skip(2).copied().collect();
            let abs = resolve_path(&self.cwd, path);
            match crate::elf::spawn_elf_process_suspended_with_args(&abs, &exec_args) {
                Ok(pid) => {
                    if self.configure_process_tty(pid, pid) {
                        let job = crate::jobs::start_process("Process", &abs, pid);
                        self.set_fg(FG_ACCENT);
                        self.print_str("job #");
                        self.set_fg(FG_OUTPUT);
                        self.print_u64(job);
                        self.print_str(" pid=");
                        self.print_u64(pid as u64);
                        self.print_str(" tty=");
                        self.print_u64(self.tty_id);
                        self.print_char('\n');
                        crate::scheduler::unblock(pid);
                    } else {
                        let _ = crate::scheduler::kill_task(pid, 143);
                    }
                }
                Err(err) => {
                    self.set_fg(FG_ERROR);
                    self.print_str("job: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(err.as_str());
                    self.print_char('\n');
                }
            }
            return;
        }

        let Some(id_text) = args.get(1).copied() else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: job <cancel|pause|resume> <id|last>\n");
            return;
        };
        let Some(id) = parse_job_id(id_text) else {
            self.set_fg(FG_ERROR);
            self.print_str("job: no such job\n");
            return;
        };
        let ok = match op {
            "cancel" => crate::jobs::cancel(id),
            "pause" => crate::jobs::pause(id),
            "resume" => crate::jobs::resume(id),
            _ => false,
        };
        self.print_bool("job", ok);
    }

    fn configure_process_tty(&mut self, pid: usize, group: usize) -> bool {
        if let Err(err) = crate::scheduler::set_process_group(pid, group) {
            self.set_fg(FG_ERROR);
            self.print_str("tty: setpgid failed: ");
            self.set_fg(FG_OUTPUT);
            self.print_str(err.as_str());
            self.print_char('\n');
            return false;
        }
        if let Err(err) = crate::scheduler::set_task_tty(pid, Some(self.tty_id)) {
            self.set_fg(FG_ERROR);
            self.print_str("tty: attach failed: ");
            self.set_fg(FG_OUTPUT);
            self.print_str(err.as_str());
            self.print_char('\n');
            return false;
        }
        true
    }

    fn begin_foreground(&mut self, pid: usize, group: usize, job_id: Option<u64>, title: &str) {
        crate::tty::reset_input_mode(self.tty_id);
        crate::tty::set_foreground_group(self.tty_id, Some(group));
        self.foreground_job = Some(ForegroundJob {
            group,
            pid,
            job_id,
            title: String::from(title),
        });
        self.set_fg(FG_ACCENT);
        self.print_str("foreground ");
        self.set_fg(FG_OUTPUT);
        self.print_str(title);
        self.print_str(" pid=");
        self.print_u64(pid as u64);
        self.print_str(" pgid=");
        self.print_u64(group as u64);
        self.print_str(" tty=");
        self.print_u64(self.tty_id);
        self.print_char('\n');
    }

    fn cmd_tty(&mut self) {
        self.set_fg(FG_ACCENT);
        self.print_str("tty #");
        self.set_fg(FG_OUTPUT);
        self.print_u64(self.tty_id);
        self.print_str(" foreground pgid=");
        match crate::tty::foreground_group(self.tty_id) {
            Some(group) => self.print_u64(group as u64),
            None => self.print_char('-'),
        }
        let active = self
            .foreground_job
            .as_ref()
            .map(|job| (job.pid, job.job_id));
        if let Some((pid, job_id)) = active {
            self.print_str(" pid=");
            self.print_u64(pid as u64);
            if let Some(job_id) = job_id {
                self.print_str(" job #");
                self.print_u64(job_id);
            }
        }
        self.print_char('\n');
        for line in crate::tty::lines() {
            self.set_fg(FG_DIM);
            self.print_str(&line);
            self.print_char('\n');
        }
    }

    fn cmd_fg(&mut self, id_text: Option<&str>) {
        if self.foreground_job.is_some() {
            self.set_fg(FG_ERROR);
            self.print_str("fg: terminal already has a foreground job\n");
            return;
        }
        let id_text = id_text.unwrap_or("last");
        let Some(id) = parse_job_id(id_text) else {
            self.set_fg(FG_ERROR);
            self.print_str("fg: no such job\n");
            return;
        };
        let Some(pid) = crate::jobs::process_id(id) else {
            self.set_fg(FG_ERROR);
            self.print_str("fg: job has no process\n");
            return;
        };
        let group = crate::scheduler::get_process_group(pid).unwrap_or(pid);
        if crate::scheduler::set_task_tty(pid, Some(self.tty_id)).is_err() {
            self.set_fg(FG_ERROR);
            self.print_str("fg: could not attach tty\n");
            return;
        }
        let _ = crate::jobs::resume(id);
        self.begin_foreground(pid, group, Some(id), "job");
    }

    fn cmd_bg(&mut self, id_text: Option<&str>) {
        let id_text = id_text.unwrap_or("last");
        let Some(id) = parse_job_id(id_text) else {
            self.set_fg(FG_ERROR);
            self.print_str("bg: no such job\n");
            return;
        };
        let Some(pid) = crate::jobs::process_id(id) else {
            self.set_fg(FG_ERROR);
            self.print_str("bg: job has no process\n");
            return;
        };
        let _ = crate::scheduler::set_task_tty(pid, Some(self.tty_id));
        if crate::jobs::resume(id) {
            self.set_fg(FG_ACCENT);
            self.print_str("background job #");
            self.set_fg(FG_OUTPUT);
            self.print_u64(id);
            self.print_str(" pid=");
            self.print_u64(pid as u64);
            self.print_char('\n');
        } else {
            self.set_fg(FG_ERROR);
            self.print_str("bg: resume failed\n");
        }
    }

    fn cmd_notify(&mut self, op: Option<&str>, arg: Option<&str>) {
        match (op, arg) {
            (None, _) | (Some("history"), _) => self.cmd_lines(
                "NOTIFICATION HISTORY",
                crate::notifications::history_lines(),
            ),
            (Some("dismiss"), Some(id)) => {
                let ok = parse_u64(id)
                    .map(crate::notifications::dismiss)
                    .unwrap_or(false);
                self.print_bool("notify", ok);
            }
            (Some("group"), Some(title)) => {
                let count = crate::notifications::dismiss_group(title);
                self.set_fg(FG_ACCENT);
                self.print_str("dismissed ");
                self.print_u64(count as u64);
                self.print_str(" notification(s)\n");
            }
            (Some("clear"), _) => {
                crate::notifications::clear();
                self.set_fg(FG_ACCENT);
                self.print_str("notifications cleared\n");
            }
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: notify [history|dismiss <id>|group <title>|clear]\n");
            }
        }
    }

    fn print_result(&mut self, prefix: &str, result: Result<(), &'static str>) {
        match result {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str(prefix);
                self.print_str(": ok\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str(prefix);
                self.print_str(": ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err);
                self.print_char('\n');
            }
        }
    }

    fn print_bool(&mut self, prefix: &str, ok: bool) {
        if ok {
            self.set_fg(FG_ACCENT);
            self.print_str(prefix);
            self.print_str(": ok\n");
        } else {
            self.set_fg(FG_ERROR);
            self.print_str(prefix);
            self.print_str(": not found\n");
        }
    }

    fn require_admin(&mut self, prefix: &str) -> bool {
        match crate::security::require_admin() {
            Ok(()) => true,
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str(prefix);
                self.print_str(": ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
                false
            }
        }
    }

    fn require_install_mutation(&mut self, prefix: &str) -> bool {
        if crate::fw_cfg::installer_mode() {
            return true;
        }
        self.require_admin(prefix)
    }

    fn cmd_power(&mut self, op: Option<&str>) {
        match op {
            Some("reboot") => crate::acpi::reboot(),
            Some("shutdown") => self.print_power_result(crate::acpi::shutdown()),
            Some("sleep") => self.print_power_result(crate::acpi::sleep()),
            _ => self.cmd_lines("POWER", crate::acpi::status_lines()),
        }
    }

    fn print_power_result(&mut self, result: Result<(), &'static str>) {
        match result {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("power operation requested\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("power: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err);
                self.print_char('\n');
            }
        }
    }

    fn cmd_log(&mut self) {
        let _ = crate::klog::flush_to_disk();
        self.cmd_lines("KERNEL LOG", crate::klog::lines());
    }

    fn cmd_fsck(&mut self) {
        match crate::coolfs::check() {
            Some(report) => {
                self.set_fg(if report.ok { FG_ACCENT } else { FG_WARN });
                self.print_str(if report.ok {
                    "coolfs root ok\n"
                } else {
                    "coolfs root warning\n"
                });
                self.set_fg(FG_OUTPUT);
                self.print_str("root entries ");
                self.print_u64(report.root_entries as u64);
                self.print_str("  blocks ");
                self.print_u64(report.stats.used_blocks as u64);
                self.print_char('/');
                self.print_u64(report.stats.total_blocks as u64);
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("coolfs: unable to read root filesystem\n");
            }
        }
        match crate::fat32::check() {
            Some(report) => {
                self.set_fg(if report.ok { FG_ACCENT } else { FG_WARN });
                self.print_str(if report.ok {
                    "legacy fat32 ok\n"
                } else {
                    "legacy fat32 warning\n"
                });
                self.set_fg(FG_OUTPUT);
                self.print_str("root entries ");
                self.print_u64(report.root_entries as u64);
                self.print_str("  clusters ");
                self.print_u64(report.stats.used_clusters as u64);
                self.print_char('/');
                self.print_u64(report.stats.total_clusters as u64);
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_WARN);
                self.print_str("legacy fat32: unavailable\n");
            }
        }
    }

    fn cmd_df(&mut self) {
        self.set_fg(FG_ACCENT);
        self.print_str("Filesystem  Used  Free  Total\n");
        self.set_fg(FG_OUTPUT);
        if let Some(cool) = crate::coolfs::stats() {
            let cool_used = cool.used_blocks as usize * cool.block_size as usize;
            let cool_free = cool.free_blocks as usize * cool.block_size as usize;
            let cool_total = cool.total_blocks as usize * cool.block_size as usize;
            self.print_str("coolfs:/    ");
            self.print_size(cool_used);
            self.print_str("  ");
            self.print_size(cool_free);
            self.print_str("  ");
            self.print_size(cool_total);
            self.print_char('\n');
        }
        if let Some(stats) = crate::fat32::stats() {
            let free = stats.free_clusters as usize * stats.bytes_per_cluster as usize;
            let used = stats.used_clusters as usize * stats.bytes_per_cluster as usize;
            let total = stats.total_clusters as usize * stats.bytes_per_cluster as usize;
            self.print_str("fat32:/FAT  ");
            self.print_size(used);
            self.print_str("  ");
            self.print_size(free);
            self.print_str("  ");
            self.print_size(total);
            self.print_char('\n');
        }
    }

    fn cmd_info(&mut self) {
        let heap = crate::allocator::heap_snapshot();
        let pressure = crate::memory_pressure::snapshot();
        let task_stats = crate::scheduler::resource_stats();

        self.set_fg(FG_ACCENT);
        self.print_str("Heap  : ");
        self.set_fg(FG_OUTPUT);
        self.print_size(heap.used);
        self.set_fg(FG_DIM);
        self.print_str(" / ");
        self.set_fg(FG_OUTPUT);
        self.print_size(heap.total);
        self.set_fg(FG_DIM);
        self.print_str(" ");
        self.print_str(pressure.level.as_str());
        self.print_char('\n');

        self.set_fg(FG_ACCENT);
        self.print_str("Tasks : ");
        self.set_fg(FG_OUTPUT);
        self.print_u64(task_stats.active_tasks as u64);
        self.set_fg(FG_DIM);
        self.print_str(" / ");
        self.set_fg(FG_OUTPUT);
        self.print_u64(task_stats.max_active_tasks as u64);
        self.set_fg(FG_DIM);
        self.print_str(" active");
        self.print_char('\n');

        let cpuid = raw_cpuid::CpuId::new();
        if let Some(v) = cpuid.get_vendor_info() {
            self.set_fg(FG_ACCENT);
            self.print_str("CPU   : ");
            self.set_fg(FG_OUTPUT);
            self.print_str(v.as_str());
            self.print_char('\n');
        }
        if let Some(b) = cpuid.get_processor_brand_string() {
            self.set_fg(FG_ACCENT);
            self.print_str("Brand : ");
            self.set_fg(FG_OUTPUT);
            self.print_str(b.as_str().trim());
            self.print_char('\n');
        }

        self.set_fg(FG_ACCENT);
        self.print_str("CWD   : ");
        self.set_fg(FG_DIR);
        let cwd = self.cwd.clone();
        self.print_str(&cwd);
        self.print_char('\n');
    }

    fn cmd_uptime(&mut self) {
        let ticks = crate::interrupts::ticks();
        let secs = crate::interrupts::uptime_secs();
        let mins = secs / 60;
        let hours = mins / 60;
        let s = secs % 60;
        let m = mins % 60;

        self.set_fg(FG_ACCENT);
        self.print_str("Up: ");
        self.set_fg(FG_OUTPUT);
        self.print_u64(hours);
        self.print_char(':');
        if m < 10 {
            self.print_char('0');
        }
        self.print_u64(m);
        self.print_char(':');
        if s < 10 {
            self.print_char('0');
        }
        self.print_u64(s);
        self.set_fg(FG_DIM);
        self.print_str("  (");
        self.print_u64(ticks);
        self.print_str(" ticks)\n");
    }

    // ── Rendering helpers ─────────────────────────────────────────────────────
}
