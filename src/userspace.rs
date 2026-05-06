/// Boot-time userspace probes.
///
/// Phase 32 runs the sentinel checks from normal ELF binaries instead of
/// jumping ring 3 into kernel `.text`. Kernel mappings can therefore remain
/// supervisor-only while the boot smoke output stays stable.

pub fn spawn_user_process(pid: u64) -> bool {
    let arg = if pid == 1 { "1" } else { "2" };
    match crate::elf::spawn_elf_process_with_args("/bin/sentinel", &[arg]) {
        Ok(_) => true,
        Err(err) => {
            crate::println!("[userspace] sentinel spawn failed: {}", err.as_str());
            false
        }
    }
}
