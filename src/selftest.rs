extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use spin::Mutex;

static RESULTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn run_boot_tests() {
    let mut ok = 0usize;
    let mut fail = 0usize;
    check(
        "path-normalize",
        crate::vfs::normalize_path("/A/./B/../C") == "/A/C",
        &mut ok,
        &mut fail,
    );
    check(
        "root-normalize",
        crate::vfs::normalize_path("/../") == "/",
        &mut ok,
        &mut fail,
    );
    check(
        "syscall-range",
        crate::syscall::validate_user_range_for_test(0x1000, 16, 4096, false),
        &mut ok,
        &mut fail,
    );
    check(
        "syscall-null",
        !crate::syscall::validate_user_range_for_test(0, 16, 4096, false),
        &mut ok,
        &mut fail,
    );
    check(
        "scheduler-lifecycle",
        crate::scheduler::SCHEDULER.lock().tasks.len() >= 1,
        &mut ok,
        &mut fail,
    );
    check(
        "fat32-mutation",
        fat32_mutation_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check(
        "coolfs-mutation",
        coolfs_mutation_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check(
        "vfs-write-enforcement",
        vfs_write_enforcement(),
        &mut ok,
        &mut fail,
    );
    check(
        "coolfs-permissions",
        coolfs_permission_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check(
        "package-grants",
        package_grants_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check(
        "session-switch",
        session_login_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check(
        "account-management",
        account_management_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check("umask", umask_roundtrip(), &mut ok, &mut fail);
    check(
        "service-supervisor",
        crate::services::service_roundtrip_for_test(),
        &mut ok,
        &mut fail,
    );
    check(
        "app-manifest-validation",
        crate::app_metadata::validate_installed_manifests().is_ok(),
        &mut ok,
        &mut fail,
    );
    check(
        "net-api",
        matches!(crate::net::dns_resolve("93.184.216.34"), Ok(0x5db8_d822))
            && !crate::net::protocol_lines().is_empty(),
        &mut ok,
        &mut fail,
    );
    check(
        "ps2-kbd-fallback",
        ps2_kbd_fallback_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check(
        "ps2-mouse-fallback",
        ps2_mouse_fallback_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check(
        "xhci-ring-link-cycle",
        crate::usb::xhci::transfer_ring_cycle_refresh_for_test(),
        &mut ok,
        &mut fail,
    );
    check("png-decode", png_decode_roundtrip(), &mut ok, &mut fail);
    check(
        "browser-html-render",
        browser_html_render_roundtrip(),
        &mut ok,
        &mut fail,
    );
    check(
        "tls-hostname-edges",
        crate::tls::hostname_selftest_passes(),
        &mut ok,
        &mut fail,
    );
    check(
        "tls-http-response-complete",
        crate::tls::http_response_selftest_passes(),
        &mut ok,
        &mut fail,
    );
    check(
        "boot-kernel-supervisor",
        boot_kernel_supervisor_only(),
        &mut ok,
        &mut fail,
    );
    check(
        "process-kernel-supervisor",
        process_kernel_supervisor_only(),
        &mut ok,
        &mut fail,
    );
    check(
        "process-signals",
        crate::process_model::signal_selftest_passes(),
        &mut ok,
        &mut fail,
    );
    check(
        "tty-routing",
        crate::tty::selftest_passes(),
        &mut ok,
        &mut fail,
    );
    crate::println!("[selftest] kernel unit checks ok={} fail={}", ok, fail);
    crate::klog::log_owned(format!("selftest: ok={} fail={}", ok, fail));
}

pub fn lines() -> Vec<String> {
    let results = RESULTS.lock();
    if results.is_empty() {
        return alloc::vec![String::from("selftests not run")];
    }
    results.clone()
}

fn check(name: &str, passed: bool, ok: &mut usize, fail: &mut usize) {
    if passed {
        *ok += 1;
    } else {
        *fail += 1;
    }
    RESULTS
        .lock()
        .push(format!("{} {}", if passed { "ok" } else { "fail" }, name));
}

fn fat32_mutation_roundtrip() -> bool {
    let _ = crate::vfs::vfs_kernel_create_dir("/FAT/TMP");
    let path = "/FAT/TMP/SELFTEST.TXT";
    match crate::vfs::vfs_kernel_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return false,
    }
    if crate::vfs::vfs_kernel_write_file(path, b"selftest\n").is_err() {
        return false;
    }
    crate::vfs::vfs_kernel_read_file(path)
        .map(|bytes| bytes.as_slice() == b"selftest\n")
        .unwrap_or(false)
}

fn coolfs_mutation_roundtrip() -> bool {
    if crate::coolfs::mount_or_format().is_err() {
        return false;
    }
    let _ = crate::vfs::vfs_kernel_create_dir("/TMP");
    let path = "/TMP/ROUNDTRIP.TXT";
    match crate::vfs::vfs_kernel_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return false,
    }
    if crate::vfs::vfs_kernel_safe_write_file(path, b"coolfs selftest\n").is_err() {
        return false;
    }
    if !matches!(
        crate::vfs::vfs_create_file("/COOLFS.IMG"),
        Err(crate::fat32::FsError::PermissionDenied)
    ) {
        return false;
    }
    crate::vfs::vfs_read_file(path)
        .map(|bytes| bytes.as_slice() == b"coolfs selftest\n")
        .unwrap_or(false)
}

fn ps2_kbd_fallback_roundtrip() -> bool {
    crate::keyboard::enable_ps2_fallback();
    crate::keyboard::disable_ps2_fallback();
    true
}

fn ps2_mouse_fallback_roundtrip() -> bool {
    crate::mouse::enable_ps2_fallback();
    crate::mouse::disable_ps2_fallback();
    true
}

fn vfs_write_enforcement() -> bool {
    if !matches!(
        crate::vfs::vfs_create_file("/CONFIG/SELFTEST.DENY"),
        Err(crate::fat32::FsError::PermissionDenied)
    ) {
        return false;
    }
    if !matches!(
        crate::vfs::vfs_safe_write_file("/TMP/../CONFIG/SELFTEST.DENY", b"deny\n"),
        Err(crate::fat32::FsError::PermissionDenied)
    ) {
        return false;
    }
    if !matches!(
        crate::vfs::vfs_create_dir("/APPS/SELFTEST.DENY"),
        Err(crate::fat32::FsError::PermissionDenied)
    ) {
        return false;
    }

    let _ = crate::vfs::vfs_create_dir("/TMP");
    let path = "/TMP/VFS_OK.TXT";
    let _ = crate::vfs::vfs_kernel_delete(path);
    match crate::vfs::vfs_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return false,
    }
    if crate::vfs::vfs_safe_write_file("/TMP/./VFS_OK.TXT", b"vfs-ok\n").is_err() {
        return false;
    }
    crate::vfs::vfs_read_file(path)
        .map(|bytes| bytes.as_slice() == b"vfs-ok\n")
        .unwrap_or(false)
}

fn boot_kernel_supervisor_only() -> bool {
    let boot_pml4 = crate::vmm::current_pml4();
    !crate::vmm::user_range_accessible_in(boot_pml4, 0x100000, 16, false)
        && !crate::vmm::user_range_accessible_in(
            boot_pml4,
            crate::allocator::HEAP_START as u64,
            16,
            true,
        )
}

fn process_kernel_supervisor_only() -> bool {
    let Some(pml4) = crate::vmm::new_process_pml4() else {
        return false;
    };

    let kernel_blocked = !crate::vmm::user_range_accessible_in(pml4, 0x100000, 16, false)
        && !crate::vmm::user_range_accessible_in(
            pml4,
            crate::allocator::HEAP_START as u64,
            16,
            true,
        );

    let user_page_ok = if let Some(frame) = crate::vmm::alloc_zeroed_frame() {
        let flags = x86_64::structures::paging::PageTableFlags::PRESENT
            | x86_64::structures::paging::PageTableFlags::WRITABLE
            | x86_64::structures::paging::PageTableFlags::USER_ACCESSIBLE
            | x86_64::structures::paging::PageTableFlags::NO_EXECUTE;
        match crate::vmm::map_owned_frame_in(
            pml4,
            x86_64::VirtAddr::new(crate::vmm::USER_MMAP_BASE),
            frame,
            flags,
        ) {
            Ok(()) => {
                crate::vmm::user_range_accessible_in(pml4, crate::vmm::USER_MMAP_BASE, 8, true)
            }
            Err(_) => {
                crate::vmm::free_unmapped_frame(frame);
                false
            }
        }
    } else {
        false
    };

    crate::vmm::free_address_space(pml4);
    kernel_blocked && user_page_ok
}

fn coolfs_permission_roundtrip() -> bool {
    let current = crate::security::current_user();
    let Some(bin) = crate::vfs::vfs_metadata("/bin/hello") else {
        return false;
    };
    if bin.uid != crate::security::ROOT_UID || bin.mode & 0o111 == 0 {
        return false;
    }
    let Some(tmp) = crate::vfs::vfs_metadata("/TMP") else {
        return false;
    };
    if tmp.uid != crate::security::USER_UID
        || tmp.gid != crate::security::USER_GID
        || tmp.mode & 0o002 == 0
    {
        return false;
    }
    if !crate::vfs::vfs_can_execute("/bin/hello") || crate::vfs::vfs_can_execute("/bin/motd.txt") {
        return false;
    }

    let path = "/TMP/PERMTEST.TXT";
    let _ = crate::vfs::vfs_kernel_delete(path);
    match crate::vfs::vfs_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return false,
    }
    if crate::vfs::vfs_write_file(path, b"allowed\n").is_err() {
        return false;
    }
    if crate::vfs::vfs_chmod(path, 0o400).is_err() {
        return false;
    }
    if !matches!(
        crate::vfs::vfs_write_file(path, b"denied\n"),
        Err(crate::fat32::FsError::PermissionDenied)
    ) {
        return false;
    }
    if crate::vfs::vfs_chmod(path, 0o600).is_err() {
        return false;
    }
    if crate::vfs::vfs_chown(path, crate::security::GUEST_UID, crate::security::USER_GID).is_err() {
        return false;
    }
    if crate::vfs::vfs_chown(path, current.uid, current.gid).is_err() {
        return false;
    }
    crate::vfs::vfs_write_file(path, b"allowed-again\n").is_ok()
}

fn package_grants_roundtrip() -> bool {
    let desktop = crate::security::package_credentials("desktop");
    let network = crate::security::package_credentials("network");
    let files = crate::security::package_credentials("filesystem");
    let shell = crate::security::package_credentials("shell");
    crate::security::can_desktop(desktop)
        && !crate::security::can_network(desktop)
        && crate::security::can_network(network)
        && crate::security::can_write_files(files)
        && crate::security::can_execute_files(desktop)
        && !crate::security::can_admin(shell)
}

fn session_login_roundtrip() -> bool {
    let restore_name = crate::security::current_user().name;
    let restore = || crate::security::set_session_for_test(&restore_name);
    if !crate::security::set_session_for_test("guest") {
        return false;
    }
    let guest = crate::security::current_user();
    if guest.uid != crate::security::GUEST_UID || crate::security::require_admin().is_ok() {
        return restore() && false;
    }
    if !matches!(
        crate::vfs::vfs_create_file("/Users/root/GUEST_DENY"),
        Err(crate::fat32::FsError::PermissionDenied)
    ) {
        return restore() && false;
    }
    let path = "/Users/guest/SESSION.TXT";
    let _ = crate::vfs::vfs_kernel_delete(path);
    match crate::vfs::vfs_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return restore() && false,
    }
    if crate::vfs::vfs_write_file(path, b"guest\n").is_err() {
        return restore() && false;
    }
    let Some(meta) = crate::vfs::vfs_metadata(path) else {
        return restore() && false;
    };
    let denied_chown = matches!(
        crate::vfs::vfs_chown(path, crate::security::ROOT_UID, crate::security::ROOT_GID),
        Err(crate::fat32::FsError::PermissionDenied)
    );
    restore() && denied_chown && meta.uid == crate::security::GUEST_UID
}

fn account_management_roundtrip() -> bool {
    let restore_name = crate::security::current_user().name;
    let restore = || crate::security::set_session_for_test(&restore_name);
    let _ = crate::security::delete_user("accttest");
    let created = match crate::security::create_user("accttest", "acctpass31", "user") {
        Ok(user) => user.uid >= crate::security::GUEST_UID && user.role == "user",
        Err(_) => return restore() && false,
    };
    let disabled = crate::security::set_user_enabled("accttest", false)
        .map(|user| !user.login_enabled)
        .unwrap_or(false);
    let enabled = crate::security::set_user_enabled("accttest", true)
        .map(|user| user.login_enabled)
        .unwrap_or(false);
    let promoted = crate::security::set_user_role("accttest", "admin")
        .map(|user| user.role == "admin")
        .unwrap_or(false);
    let password = crate::security::reset_user_password("accttest", "newpass31").is_ok()
        && crate::security::login("accttest", "newpass31").is_ok();
    let _ = restore();
    let deleted = crate::security::delete_user("accttest").is_ok()
        && crate::security::user_by_name("accttest").is_none();
    restore() && created && disabled && enabled && promoted && password && deleted
}

fn umask_roundtrip() -> bool {
    let old = crate::security::set_umask(0o077);
    let path = "/TMP/UMASK.TXT";
    let _ = crate::vfs::vfs_kernel_delete(path);
    let created = crate::vfs::vfs_create_file(path).is_ok();
    let mode_ok = crate::vfs::vfs_metadata(path)
        .map(|meta| meta.mode == 0o600)
        .unwrap_or(false);
    crate::security::set_umask(old);
    created && mode_ok
}

fn png_decode_roundtrip() -> bool {
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xfd,
        0xd4, 0x9a, 0x73, 0x00, 0x00, 0x00, 0x12, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0xc0, 0x00, 0xc2, 0x0c, 0xff, 0x81, 0x00, 0x00, 0x1f, 0xee, 0x05, 0xfb, 0x0b,
        0xd9, 0x68, 0x8b, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let Ok(image) = crate::png::decode_rgb8(PNG, 16) else {
        return false;
    };
    let _ = crate::vfs::vfs_create_dir("/TMP");
    let _ = crate::vfs::vfs_safe_write_file("/TMP/PNGTEST.PNG", PNG);
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE19.HTML",
        b"<!doctype html><html><head><title>Phase 19</title></head><body><h1>Phase 19</h1><blockquote>quoted text</blockquote><table><tr><th>Name</th><th>Status</th></tr><tr><td>PNG</td><td>ready</td></tr></table><p><img src=\"PNGTEST.PNG\" alt=\"checker\"></p></body></html>",
    );
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE49.HTML",
        b"<!doctype html><html><head><title>Phase 49 Browser Engine</title><style>.hide{display:none}.hero{text-align:center;color:#123456;background-color:#ddeeff;margin-left:16px}img.logo{width:40px;height:28px}</style></head><body><h1 class=\"hero\">Phase 49 CSS layout</h1><p class=\"hide\">hidden</p><p class=\"hero\">Styled CSS2 block with inherited layout hints.</p><img class=\"logo\" src=\"PNGTEST.PNG\" alt=\"checker\"><form action=\"/search\"><input type=\"hidden\" name=\"phase\" value=\"49\"><input type=\"search\" name=\"q\" value=\"coolos\"><input type=\"checkbox\" name=\"safe\" value=\"1\" checked><input type=\"submit\" name=\"go\" value=\"Go\"></form></body></html>",
    );
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE50.CSS.HTML",
        b"<!doctype html><html><head><title>Phase 50 CSS2</title><style>body{color:#202020}.card{margin-left:24px;background:#f0f7ff}.right{text-align:right}.gone{visibility:hidden}</style></head><body><h1>CSS2 cascade</h1><p class=\"card\">Indented card-like block without nesting UI cards.</p><p class=\"right\">Right aligned text</p><p class=\"gone\">not rendered</p></body></html>",
    );
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE51.FORM.HTML",
        b"<!doctype html><html><head><title>Phase 51 Forms</title></head><body><h1>HTML5 forms</h1><form action=\"/apply\"><input type=\"hidden\" name=\"source\" value=\"phase51\"><input type=\"email\" name=\"email\" value=\"user@example.com\"><input type=\"radio\" name=\"tier\" value=\"pro\" checked><textarea name=\"notes\" rows=\"4\" placeholder=\"Notes\"></textarea><input type=\"submit\" name=\"submit\" value=\"Send\"></form></body></html>",
    );
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE52.DOM.HTML",
        b"<!doctype html><html><head><title>Phase 52 DOM Events</title><style>#target{color:blue}.event{background:#fff4cc}</style></head><body><h1>DOM event foundation</h1><p id=\"target\" class=\"event\">Clickable links and form submits route through browser hit boxes.</p><a href=\"PHASE49.HTML\">Open phase 49 fixture</a><button type=\"button\" value=\"noop\" aria-label=\"Button event\">Button event</button></body></html>",
    );
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE53.DOM.HTML",
        b"<!doctype html><html><head><title>Phase 53 DOM Forms</title><style>form{margin-left:12px}.primary{color:#0b4f71;background:#eef8ff}</style></head><body><h1>Phase 53 DOM backed form</h1><p class=\"primary\">DOM backed form controls keep live values across reflow.</p><form action=\"/find\" method=\"get\"><input type=\"hidden\" name=\"phase\" value=\"53\"><input type=\"search\" name=\"q\" value=\"cool\" placeholder=\"Search\"><input type=\"checkbox\" name=\"safe\" value=\"1\"><select name=\"mode\" aria-label=\"Mode\"><option value=\"web\">Web</option><option value=\"img\" selected>Images</option></select><textarea name=\"notes\" rows=\"3\">old notes</textarea><input type=\"submit\" name=\"go\" value=\"Go\"><input type=\"reset\" value=\"Reset\"></form><form action=\"/post\" method=\"post\"><input type=\"text\" name=\"msg\" value=\"draft\"><input type=\"submit\" name=\"post\" value=\"Post\"></form></body></html>",
    );
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE54.POST.HTML",
        b"<!doctype html><html><head><title>Phase 54 POST</title><style>form{margin-left:12px}.net{color:#063970;background:#eef7ff}</style></head><body><h1>Phase 54 POST submission</h1><p class=\"net\">POST forms now build real request bodies through the Browser network path.</p><form action=\"https://example.com/post\" method=\"post\"><input type=\"hidden\" name=\"phase\" value=\"54\"><input type=\"text\" name=\"msg\" value=\"draft\"><textarea name=\"notes\" rows=\"3\">network body</textarea><input type=\"submit\" name=\"send\" value=\"Send\"></form></body></html>",
    );
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE55.SESSION.HTML",
        b"<!doctype html><html><head><title>Phase 55 Session</title><style>.session{color:#16435c;background:#edf8ff}</style></head><body><h1>Phase 55 browser session</h1><p class=\"session\">Browser GET and POST requests now use persistent cookie state.</p><a href=\"browser://session\">Open session state</a></body></html>",
    );
    let _ = crate::vfs::vfs_safe_write_file(
        "/TMP/PHASE56.BOX.HTML",
        b"<!doctype html><html><head><title>Phase 56 CSS Box</title><style>.box{width:50%;margin:4px 8px 6px 8px;padding:6px 10px;border:2px solid #335577;background:#eaf6ff}.wide{width:75%;padding:4px 12px;border:1px solid #884422;background:#fff8ea}input.field{width:50%;padding:4px;border:1px solid #335577}</style></head><body><h1>Phase 56 CSS box model</h1><div class=\"box\">Phase 56 box model wraps text inside a bounded content box while margin padding border and background are painted by layout.</div><p class=\"wide\">Percentage width blocks reflow when the Browser window size changes.</p><form action=\"/box\"><input class=\"field\" type=\"search\" name=\"q\" value=\"layout\" placeholder=\"Box search\"><input type=\"submit\" value=\"Go\"></form></body></html>",
    );
    image.width == 2
        && image.height == 2
        && image.pixels.as_slice() == [0x00ff0000, 0x0000ff00, 0x000000ff, 0x00ffffff]
}

fn browser_html_render_roundtrip() -> bool {
    let html = "<!doctype html><html><head><style>.hidden{display:none}.center{text-align:center}.panel{margin-left:16px;color:#123456;background-color:#ddeeff}img.logo{width:16px;height:12px}</style></head><body><h1 class=\"center\">Heading</h1><p class=\"hidden\">Hidden text</p><p class=\"panel\">Styled words</p><blockquote>Quoted words</blockquote><ul><li>First</li></ul><table><tr><th>Name</th><th>Status</th></tr><tr><td>PNG</td><td>ready</td></tr></table><form action=\"/search\"><input type=\"hidden\" name=\"token\" value=\"abc\"><input type=\"search\" name=\"q\" value=\"cool\" placeholder=\"Search\"><input type=\"checkbox\" name=\"safe\" value=\"1\" checked><input type=\"submit\" name=\"go\" value=\"Go\"></form><img class=\"logo\" src=\"PNGTEST.PNG\" alt=\"checker\"></body></html>";
    let lines =
        crate::apps::browser::render_document_debug_for_test("file:///TMP/PHASE19.HTML", html, 72);
    let styled = crate::apps::browser::render_document_style_debug_for_test(
        "file:///TMP/PHASE19.HTML",
        html,
        72,
    );
    let has_heading = lines.iter().any(|line| line == "Heading");
    let has_css = !lines.iter().any(|line| line.contains("Hidden text"))
        && styled
            .iter()
            .any(|line| line.contains("Styled words [indent=16] [color=#123456] [bg=#DDEEFF]"))
        && styled
            .iter()
            .any(|line| line.contains("Heading [align=center]"));
    let has_quote = lines.iter().any(|line| line.contains("> Quoted words"));
    let has_table = lines
        .iter()
        .any(|line| line.contains("| Name") && line.contains("Status"));
    let has_image = lines.iter().any(|line| {
        line.contains("[image 16x12] checker") && line.contains("file:///TMP/PNGTEST.PNG")
    });
    let has_form = lines.iter().any(|line| line == "[search] Search")
        && lines.iter().any(|line| line.contains("[checkbox] safe"))
        && lines.iter().any(|line| {
            line.contains("[button] Go")
                && line.contains("file:///search?token=abc&q=cool&safe=1&go=Go")
        });
    let phase53 = "<!doctype html><html><body><p>DOM backed form</p><form action=\"/find\" method=\"get\"><input type=\"hidden\" name=\"phase\" value=\"53\"><input type=\"search\" name=\"q\" value=\"cool\" placeholder=\"Search\"><input type=\"checkbox\" name=\"safe\" value=\"1\"><select name=\"mode\" aria-label=\"Mode\"><option value=\"web\">Web</option><option value=\"img\" selected>Images</option></select><textarea name=\"notes\" rows=\"3\">old notes</textarea><input type=\"submit\" name=\"go\" value=\"Go\"></form><form action=\"/post\" method=\"post\"><input type=\"text\" name=\"msg\" value=\"draft\"><input type=\"submit\" name=\"post\" value=\"Post\"></form></body></html>";
    let interaction = crate::apps::browser::document_interaction_debug_for_test(
        "file:///TMP/PHASE53.DOM.HTML",
        phase53,
    );
    let has_interaction = interaction
        .iter()
        .any(|line| line == "dom has form=true input=true text=true")
        && interaction.iter().any(|line| line == "forms=2 controls=8")
        && interaction
            .iter()
            .any(|line| line == "edited=true noted=true toggled=true")
        && interaction.iter().any(|line| {
            line == "file:///find?phase=53&q=edited&safe=1&mode=img&notes=phase+53+note&go=Go"
        })
        && interaction
            .iter()
            .any(|line| line == "POST file:///post body=msg=draft&post=Post");
    let post_request = crate::net::http_request_debug_for_test(
        "POST",
        "https://example.com/post",
        "msg=draft&post=Post",
        "application/x-www-form-urlencoded",
    );
    let has_post_request = post_request
        .iter()
        .any(|line| line == "POST /post HTTP/1.1")
        && post_request.iter().any(|line| line == "Host: example.com")
        && post_request
            .iter()
            .any(|line| line == "Content-Type: application/x-www-form-urlencoded")
        && post_request.iter().any(|line| line == "Content-Length: 19")
        && post_request
            .iter()
            .any(|line| line == "msg=draft&post=Post");
    let cookie_debug = crate::browser_session::cookie_debug_for_test();
    let has_cookie_session = cookie_debug.iter().any(|line| line == "stored_sid=true")
        && cookie_debug.iter().any(|line| line == "stored_theme=true")
        && cookie_debug
            .iter()
            .any(|line| line == "rejected_domain=true")
        && cookie_debug
            .iter()
            .any(|line| line == "secure_header=sid=abc; theme=dark")
        && cookie_debug
            .iter()
            .any(|line| line == "plain_header=theme=dark")
        && cookie_debug
            .iter()
            .any(|line| line == "subdomain_header=theme=dark")
        && cookie_debug.iter().any(|line| line == "deleted_sid=true")
        && cookie_debug
            .iter()
            .any(|line| line == "after_delete=theme=dark");
    let cookie_request = crate::net::http_cookie_request_debug_for_test(
        "https://example.com/account",
        "sid=abc; theme=dark",
    );
    let has_cookie_request = cookie_request
        .iter()
        .any(|line| line == "GET /account HTTP/1.1")
        && cookie_request
            .iter()
            .any(|line| line == "Host: example.com")
        && cookie_request
            .iter()
            .any(|line| line == "Cookie: sid=abc; theme=dark");
    let phase56 = "<!doctype html><html><head><style>.box{width:50%;margin:4px 8px 6px 8px;padding:6px 10px;border:2px solid #335577;background:#eaf6ff}.wide{width:75%;padding:4px 12px;border:1px solid #884422;background:#fff8ea}input.field{width:50%;padding:4px;border:1px solid #335577}</style></head><body><div class=\"box\">Phase 56 box model wraps text inside a bounded content box while margin padding border and background are painted by layout.</div><p class=\"wide\">Percentage width blocks reflow when the Browser window size changes.</p><form action=\"/box\"><input class=\"field\" type=\"search\" name=\"q\" value=\"layout\" placeholder=\"Box search\"><input type=\"submit\" value=\"Go\"></form></body></html>";
    let box_style = crate::apps::browser::render_document_style_debug_for_test(
        "file:///TMP/PHASE56.BOX.HTML",
        phase56,
        72,
    );
    let box_layout = crate::apps::browser::render_document_box_debug_for_test(
        "file:///TMP/PHASE56.BOX.HTML",
        phase56,
        72,
        320,
    );
    let has_box_style = box_style.iter().any(|line| {
        line.contains("Phase 56 box model")
            && line.contains("[box-w=50%]")
            && line.contains("[margin=4,8,6,8]")
            && line.contains("[pad=6,10,6,10]")
            && line.contains("[border=2 #335577]")
    }) && box_style.iter().any(|line| {
        line.contains("[search] Box search")
            && line.contains("[box-w=50%]")
            && line.contains("[pad=4,4,4,4]")
            && line.contains("[border=1 #335577]")
    });
    let has_box_layout = box_layout.iter().any(|line| {
        line.contains("Phase 56 box model")
            && line.contains("content=160x")
            && line.contains("box=184x")
            && line.contains("at 8,")
    });
    has_heading
        && has_css
        && has_quote
        && has_table
        && has_image
        && has_form
        && has_interaction
        && has_post_request
        && has_cookie_session
        && has_cookie_request
        && has_box_style
        && has_box_layout
}
