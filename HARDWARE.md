# coolOS Hardware Compatibility Matrix

Phase 95 tracks real-PC USB boot and physical-install readiness. Use this file
to record every physical machine tested with `coolos-usb.img`,
`coolos-usb-safe.img`, or `coolos-usb-secure.img`.

## Manual QA Steps

1. Flash `target/x86_64-unknown-none/release/coolos-usb.img` to USB.
2. Boot a UEFI machine with Secure Boot disabled.
3. If the display, input, or root disk does not come up, retry with
   `coolos-usb-safe.img`.
4. If firmware supports custom Secure Boot keys, enroll the Phase 94 public
   `db` certificate from the secure USB ESP and boot `coolos-usb-secure.img`.
5. From Terminal, run:
   ```text
   hardware
   devices
   install disks
   sysreport write
   support bundle
   ```
6. Copy the relevant results from `/LOGS/HARDWARE.TXT`,
   `/LOGS/SYSREPORT.TXT`, and `/LOGS/SUPPORT-BUNDLE.TXT` into the matrix below.

## Result Labels

| Field | Labels |
| :---- | :----- |
| Framebuffer | `ok`, `safe-only`, `wrong-mode`, `blank`, `failed` |
| Input | `usb-ok`, `ps2-fallback`, `keyboard-only`, `pointer-only`, `failed` |
| Storage root | `usb-ok`, `sata-ok`, `nvme-ok`, `missing`, `failed` |
| Secure Boot | `off-ok`, `custom-db-ok`, `setup-mode`, `rejected`, `untested` |
| Install | `not-tested`, `plan-ok`, `installed-ok`, `refused-safe`, `failed` |

## Compatibility Matrix

| Machine | Firmware | Image | Secure Boot | Framebuffer | Input | Storage root | Install | Notes / Workaround |
| :------ | :------- | :---- | :---------- | :---------- | :---- | :----------- | :------ | :----------------- |
| QEMU OVMF | UEFI | `coolos-usb.img` | off-ok | ok | usb-ok | usb-ok | not-tested | Phase 95 smoke baseline |
| QEMU OVMF | UEFI | `coolos-usb-safe.img` | off-ok | safe-only | usb-ok | usb-ok | not-tested | Safe-mode smoke baseline |
| QEMU OVMF | UEFI | `coolos-usb-secure.img` | custom-db-ok | ok | usb-ok | usb-ok | not-tested | Secure Boot enrollment smoke baseline |

## Failure Notes

When a real machine fails, keep the first actionable reason from the support
bundle:

| Machine | Symptom | First failed line | Follow-up |
| :------ | :------ | :---------------- | :-------- |
| _example_ | no root disk | `boot_issue no-root failed: ...` | inspect USB MSC/UASP lines |

## Deferred Hardware Work

Phase 95 is validation and targeted hardening. It does not add Secure Boot shim
or Microsoft CA support, UASP root support, broad GPU drivers, physical MBR
installs, or destructive host-disk tooling.
