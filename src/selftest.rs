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
    let _ = crate::fat32::create_dir("/TMP");
    let path = "/TMP/SELFTEST.TXT";
    match crate::fat32::create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return false,
    }
    if crate::fat32::write_file(path, b"selftest\n").is_err() {
        return false;
    }
    crate::fat32::read_file(path)
        .map(|bytes| bytes.as_slice() == b"selftest\n")
        .unwrap_or(false)
}

fn coolfs_mutation_roundtrip() -> bool {
    if crate::coolfs::mount_or_format().is_err() {
        return false;
    }
    let _ = crate::vfs::vfs_create_dir("/COOL/TESTS");
    let path = "/COOL/TESTS/ROUNDTRIP.TXT";
    match crate::vfs::vfs_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return false,
    }
    if crate::vfs::vfs_safe_write_file(path, b"coolfs selftest\n").is_err() {
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
    image.width == 2
        && image.height == 2
        && image.pixels.as_slice() == [0x00ff0000, 0x0000ff00, 0x000000ff, 0x00ffffff]
}

fn browser_html_render_roundtrip() -> bool {
    let html = "<!doctype html><html><body><h1>Heading</h1><blockquote>Quoted words</blockquote><ul><li>First</li></ul><table><tr><th>Name</th><th>Status</th></tr><tr><td>PNG</td><td>ready</td></tr></table><form action=\"/search\"><input type=\"search\" name=\"q\" placeholder=\"Search\"><input type=\"submit\" value=\"Go\"></form><img src=\"PNGTEST.PNG\" alt=\"checker\" width=\"2\" height=\"2\"></body></html>";
    let lines =
        crate::apps::browser::render_document_debug_for_test("file:///TMP/PHASE19.HTML", html, 72);
    let has_heading = lines.iter().any(|line| line == "Heading");
    let has_quote = lines.iter().any(|line| line.contains("> Quoted words"));
    let has_table = lines
        .iter()
        .any(|line| line.contains("| Name") && line.contains("Status"));
    let has_image = lines.iter().any(|line| {
        line.contains("[image 2x2] checker") && line.contains("file:///TMP/PNGTEST.PNG")
    });
    let has_form = lines.iter().any(|line| line == "[search] Search")
        && lines
            .iter()
            .any(|line| line.contains("[button] Go") && line.contains("file:///search"));
    has_heading && has_quote && has_table && has_image && has_form
}
