# coolOS Hardware Compatibility Matrix

Phase 96 tracks real-PC USB boot and physical-install readiness. Use this file
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

## Field Result Workflow

For every real machine, record one row in the compatibility matrix and, when
something fails, one row in the field-fix tracker. The most important support
bundle line is:

```text
hardware primary_failure=<code> detail=<reason>
```

Use that line as the first failure reason. Keep the raw support bundle around
until the fix is verified, but do not paste passwords, private keys, or personal
files into this document.

## Result Labels

| Field | Labels |
| :---- | :----- |
| Framebuffer | `ok`, `safe-only`, `wrong-mode`, `blank`, `failed` |
| Input | `usb-ok`, `ps2-fallback`, `keyboard-only`, `pointer-only`, `failed` |
| Storage root | `usb-ok`, `sata-ok`, `nvme-ok`, `missing`, `failed` |
| Secure Boot | `off-ok`, `custom-db-ok`, `setup-mode`, `rejected`, `untested` |
| Install | `not-tested`, `plan-ok`, `installed-ok`, `refused-safe`, `failed` |
| Fix status | `new`, `triaged`, `fixed`, `workaround`, `deferred` |

## Known Good

| Machine | Firmware | Image | Primary failure | Notes |
| :------ | :------- | :---- | :-------------- | :---- |
| QEMU OVMF | UEFI | `coolos-usb.img` | `none` | Normal Phase 96 smoke baseline |
| QEMU OVMF | UEFI | `coolos-usb-secure.img` | `none` | Secure Boot diagnostics baseline |

## Known Failed / Workaround

| Machine | Firmware | Image | Primary failure | Workaround | Fix status |
| :------ | :------- | :---- | :-------------- | :--------- | :--------- |
| QEMU BIOS without USB input | BIOS | `bios.img` + `fs.img` | `no-input` | Attach USB keyboard/mouse/tablet for interactive use | triaged |

## Compatibility Matrix

| Machine | Firmware | Image | Secure Boot | Framebuffer | Input | Storage root | Install | Primary failure | Notes / Workaround |
| :------ | :------- | :---- | :---------- | :---------- | :---- | :----------- | :------ | :-------------- | :----------------- |
| QEMU OVMF | UEFI | `coolos-usb.img` | off-ok | ok | usb-ok | usb-ok | not-tested | none | Phase 96 smoke baseline |
| QEMU OVMF | UEFI | `coolos-usb-safe.img` | off-ok | safe-only | usb-ok | usb-ok | not-tested | safe-framebuffer | Safe-mode smoke baseline |
| QEMU OVMF | UEFI | `coolos-usb-secure.img` | custom-db-ok | ok | usb-ok | usb-ok | not-tested | none | Secure Boot enrollment smoke baseline |
| QEMU BIOS without USB input | BIOS | `bios.img` + `fs.img` | untested | ok | failed | ide raw | not-tested | no-input | Simulated field-failure smoke |

## Failure Notes

When a real machine fails, keep the first actionable reason from the support
bundle:

| Machine | Symptom | First failed line | Follow-up | Fix status |
| :------ | :------ | :---------------- | :-------- | :--------- |
| _example_ | no root disk | `hardware primary_failure=no-root ...` | inspect USB MSC/UASP lines | new |

## Deferred Hardware Work

Phase 96 is validation and targeted hardening. It does not add Secure Boot shim
or Microsoft CA support, UASP root support, broad GPU drivers, physical MBR
installs, or destructive host-disk tooling.
