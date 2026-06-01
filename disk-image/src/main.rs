use bootloader::{BiosBoot, BootConfig};
#[cfg(feature = "uefi")]
use bootloader::UefiBoot;
/// Host-side tool: wraps the kernel ELF into bootable disk images using
/// bootloader 0.11's BIOS builder and, with the `uefi` feature, UEFI builder.
///
/// Usage: disk-image <path-to-kernel-elf>
/// Writes <kernel-dir>/bios.img, optionally <kernel-dir>/uefi.img, and prints
/// the generated paths. COOLOS_IMAGE_SUFFIX can be used by fallback image
/// targets to emit sibling artifacts such as uefi-safe.img.
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let kernel_path = args.next().expect("Usage: disk-image <path-to-kernel-elf>");

    let kernel = PathBuf::from(&kernel_path);
    let out_dir = kernel.parent().unwrap_or_else(|| std::path::Path::new("."));
    let suffix = std::env::var("COOLOS_IMAGE_SUFFIX").unwrap_or_default();
    let bios_path = out_dir.join(format!("bios{}.img", suffix));

    // Request 1920x1080 by default so desktop screendumps are true 1080p.
    // Safe USB images override this through COOLOS_FB_WIDTH/HEIGHT.
    let boot_config = boot_config();

    BiosBoot::new(&kernel)
        .set_boot_config(&boot_config)
        .create_disk_image(&bios_path)
        .unwrap_or_else(|e| panic!("failed to create disk image: {}", e));

    println!("{}", bios_path.display());

    #[cfg(feature = "uefi")]
    {
        let uefi_path = out_dir.join(format!("uefi{}.img", suffix));
        UefiBoot::new(&kernel)
            .set_boot_config(&boot_config)
            .create_disk_image(&uefi_path)
            .unwrap_or_else(|e| panic!("failed to create UEFI disk image: {}", e));

        println!("{}", uefi_path.display());
    }
}

fn boot_config() -> BootConfig {
    let mut boot_config = BootConfig::default();
    boot_config.frame_buffer.minimum_framebuffer_width = Some(env_u64("COOLOS_FB_WIDTH", 1920));
    boot_config.frame_buffer.minimum_framebuffer_height =
        Some(env_u64("COOLOS_FB_HEIGHT", 1080));
    // Keep bootloader diagnostics on the debug console, but don't paint them
    // onto the visible framebuffer during normal desktop boots.
    boot_config.frame_buffer_logging = false;
    boot_config
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}
