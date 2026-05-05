.PHONY: run run-net run-usb run-usb-init run-remote run-remote-net run-vnc run-vnc-net run-headless run-headless-net run-headless-usb run-headless-usb-init smoke smoke-ui smoke-ui-ready-state smoke-framebuffer smoke-ui-goldens smoke-browser-png smoke-browser-html smoke-ui-settings smoke-ui-visual-assertions smoke-start-menu smoke-userspace-sdk smoke-userspace-gui smoke-userspace-utils smoke-net-api smoke-net-wget smoke-net-https smoke-net-https-negative smoke-net-browser-https smoke-net-browser-google smoke-usb-init smoke-hotplug-usb-init smoke-kernel-units smoke-boot-budget smoke-lowmem smoke-smp2 smoke-vga-cirrus build build-usb-init clean

TARGET  := x86_64-unknown-none.json
KERNEL  := $(CURDIR)/target/x86_64-unknown-none/release/cool_os
BIOS    := $(CURDIR)/target/x86_64-unknown-none/release/bios.img
FSIMG   := $(CURDIR)/target/x86_64-unknown-none/release/fs.img
USB_INIT_BIOS := $(BIOS)
USB_INIT_FSIMG := $(FSIMG)
QEMU_CPU ?= max
QEMU_RTC ?= -rtc base=utc,clock=host
QEMU_VNC ?= 127.0.0.1:1
USER_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/hello_user
USER_EXEC_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/exec
USER_PIPE_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/pipe
USER_READ_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/read
USER_PIPERD_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/piperd
USER_PIPEWR_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/pipewr
USER_KEYECHO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/keyecho
USER_TERMINAL_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/terminal
USER_NETDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/netdemo
USER_WGET_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/wget
USER_SDKDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/sdkdemo
USER_GUIDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/guidemo
USER_NOTES_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/notes
USER_EDITOR_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/editor
USER_TRASH_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/trash
USER_SCREENSHOT_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/screenshot
SMOKE_SECONDS ?= 18
SMOKE_FRAMEBUFFER_SECONDS ?= 30
SMOKE_INTERACTIVE_SECONDS ?= $(SMOKE_FRAMEBUFFER_SECONDS)
SMOKE_PRE_TYPE_DELAY ?= 0.8
SMOKE_USB_SECONDS ?= 18
SMOKE_BOOT_BUDGET_SECONDS ?= 8
SMOKE_VGA_SECONDS ?= 24
SMOKE_ARTIFACT_DIR ?= $(CURDIR)/target/smoke-artifacts

run: build
	@echo "Booting coolOS in QEMU..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-display cocoa \
		-debugcon stdio

run-net: build
	@echo "Booting coolOS in QEMU with virtio-net user networking..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-netdev user,id=net0 \
		-device virtio-net-pci,netdev=net0,disable-modern=on,disable-legacy=off \
		-display cocoa \
		-debugcon stdio

run-usb: build
	@echo "Booting coolOS in QEMU with xHCI-attached USB devices..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device qemu-xhci,id=xhci \
		-device usb-kbd,bus=xhci.0 \
		-device usb-mouse,bus=xhci.0 \
		-display cocoa \
		-debugcon stdio

run-usb-init: build-usb-init
	@echo "Booting coolOS in QEMU with active xHCI init..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device qemu-xhci,id=xhci \
		-device usb-kbd,bus=xhci.0 \
		-device usb-mouse,bus=xhci.0 \
		-display cocoa \
		-debugcon stdio

run-vnc: build-usb-init
	@echo "Booting coolOS in QEMU VNC with USB tablet input on $(QEMU_VNC)..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device qemu-xhci,id=xhci \
		-device usb-kbd,bus=xhci.0 \
		-device usb-tablet,bus=xhci.0 \
		-display vnc="$(QEMU_VNC)" \
		-debugcon stdio

run-vnc-net: build-usb-init
	@echo "Booting coolOS in QEMU VNC with virtio-net and USB tablet input on $(QEMU_VNC)..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device qemu-xhci,id=xhci \
		-device usb-kbd,bus=xhci.0 \
		-device usb-tablet,bus=xhci.0 \
		-netdev user,id=net0 \
		-device virtio-net-pci,netdev=net0,disable-modern=on,disable-legacy=off \
		-display vnc="$(QEMU_VNC)" \
		-debugcon stdio

run-remote: build-usb-init
	@echo "Booting coolOS in a QEMU window with USB tablet input for remote desktop..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device qemu-xhci,id=xhci \
		-device usb-kbd,bus=xhci.0 \
		-device usb-tablet,bus=xhci.0 \
		-display cocoa \
		-debugcon stdio

run-remote-net: build-usb-init
	@echo "Booting coolOS in a QEMU window with virtio-net and USB tablet input..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device qemu-xhci,id=xhci \
		-device usb-kbd,bus=xhci.0 \
		-device usb-tablet,bus=xhci.0 \
		-netdev user,id=net0 \
		-device virtio-net-pci,netdev=net0,disable-modern=on,disable-legacy=off \
		-display cocoa \
		-debugcon stdio

run-headless: build
	@echo "Booting coolOS headless in QEMU..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-display none \
		-debugcon stdio

run-headless-net: build
	@echo "Booting coolOS headless in QEMU with virtio-net user networking..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-netdev user,id=net0 \
		-device virtio-net-pci,netdev=net0,disable-modern=on,disable-legacy=off \
		-display none \
		-debugcon stdio

run-headless-usb: build
	@echo "Booting coolOS headless in QEMU with xHCI-attached USB devices..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device qemu-xhci,id=xhci \
		-device usb-kbd,bus=xhci.0 \
		-device usb-mouse,bus=xhci.0 \
		-display none \
		-debugcon stdio

run-headless-usb-init: build-usb-init
	@echo "Booting coolOS headless in QEMU with active xHCI init..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device qemu-xhci,id=xhci \
		-device usb-kbd,bus=xhci.0 \
		-device usb-mouse,bus=xhci.0 \
		-display none \
		-debugcon stdio

smoke: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--expect "[fs] /bin/hello.txt: Hello from /bin/hello.txt!" \
		--expect "[ring3 pid=1] sentinel ok" \
		--expect "[ring3 pid=2] sentinel ok" \
		--expect "[boot] desktop ready"

smoke-ui: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--expect "FB 1280x720" \
		--expect "[fs] /bin/hello.txt: Hello from /bin/hello.txt!" \
		--expect "[ring3 pid=1] sentinel ok" \
		--expect "[ring3 pid=2] sentinel ok" \
		--expect "[boot] desktop ready"

smoke-ui-ready-state: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-esc" \
		--post-hmp-delay 0.8 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-ready-state.ppm" \
		--expect-framebuffer-start-menu \
		--expect "[boot] desktop ready" \
		--expect "[ui] ready pinned=Terminal|File Manager|System Monitor|Diagnostics|Display Settings|Personalize"

smoke-framebuffer: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--screendump "$(SMOKE_ARTIFACT_DIR)/framebuffer-smoke.ppm" \
		--expect-framebuffer-desktop \
		--expect "[boot] desktop ready"

smoke-browser-png: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PNGTEST.PNG\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/browser-png-smoke.ppm" \
		--expect-framebuffer-window \
		--expect "[selftest] kernel unit checks ok=17 fail=0" \
		--expect "[boot] desktop ready"

smoke-browser-html: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE19.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/browser-html-smoke.ppm" \
		--expect-framebuffer-window \
		--expect "[selftest] kernel unit checks ok=17 fail=0" \
		--expect "[boot] desktop ready"

smoke-ui-goldens: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "ui-golden-desktop" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-golden-desktop.ppm" \
		--expect-framebuffer-desktop \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "ui-golden-file-manager" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-2" \
		--post-hmp-delay 0.8 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-golden-file-manager.ppm" \
		--expect-framebuffer-window \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "ui-golden-diagnostics" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-4" \
		--post-hmp-delay 0.8 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-golden-diagnostics.ppm" \
		--expect-framebuffer-window \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "ui-golden-crash-dialog" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-spc" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "crash dialog\n" \
		--post-hmp-delay 0.8 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-golden-crash-dialog.ppm" \
		--expect-framebuffer-dialog \
		--expect "[boot] desktop ready"

smoke-ui-settings: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-5" \
		--post-hmp-delay 0.8 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-golden-settings.ppm" \
		--expect-framebuffer-window \
		--expect "[boot] desktop ready"

smoke-ui-visual-assertions:
	python3 $(CURDIR)/scripts/ppm_visual_assert.py \
		start-menu="$(SMOKE_ARTIFACT_DIR)/start-menu-smoke.ppm" \
		settings="$(SMOKE_ARTIFACT_DIR)/ui-golden-settings.ppm" \
		diagnostics="$(SMOKE_ARTIFACT_DIR)/ui-golden-diagnostics.ppm" \
		crash-dialog="$(SMOKE_ARTIFACT_DIR)/ui-golden-crash-dialog.ppm"

smoke-start-menu: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-esc" \
		--post-hmp-delay 0.8 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/start-menu-smoke.ppm" \
		--expect-framebuffer-start-menu \
		--expect "[boot] desktop ready"

smoke-userspace-sdk: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "exec /bin/sdkdemo alpha\n" \
		--post-hmp-delay 2.0 \
		--expect "sdkdemo: libcool sdk=1 abi=5" \
		--expect "sdkdemo: argv [0]=/bin/sdkdemo [1]=alpha" \
		--expect "sdkdemo: sdk pipe ok" \
		--expect "sdkdemo: mmap ok" \
		--expect "sdkdemo: done" \
		--expect "[boot] desktop ready"

smoke-userspace-gui: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "exec /bin/guidemo\n" \
		--post-hmp-delay 2.0 \
		--expect "guidemo: window opened" \
		--expect "guidemo: presented frame" \
		--expect "[boot] desktop ready" \
		--expect-framebuffer-window

smoke-userspace-utils: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-notes" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "exec /bin/notes s\n" \
		--post-hmp-delay 2.0 \
		--expect "notes: window opened" \
		--expect "notes: saved" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-editor" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "exec /bin/editor s\n" \
		--post-hmp-delay 2.0 \
		--expect "editor: window opened" \
		--expect "editor: saved" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-trash" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "exec /bin/trash s\n" \
		--post-hmp-delay 2.0 \
		--expect "trash: window opened" \
		--expect "trash: listed" \
		--expect "trash: empty ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-screenshot" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "exec /bin/screenshot s\n" \
		--post-hmp-delay 2.0 \
		--expect "screenshot: window opened" \
		--expect "screenshot: queued /Pictures/SMOKE.PPM" \
		--expect "[boot] desktop ready"

smoke-net-api: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "exec /bin/netdemo\n" \
		--post-hmp-delay 2.0 \
		--expect "netdemo: dns example.com =" \
		--expect "GET / HTTP/1.1" \
		--expect "netdemo: http bytes" \
		--expect "[boot] desktop ready"

smoke-net-wget: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--net \
		--usb \
		--seconds 45 \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "exec /bin/wget http://example.com/\n" \
		--post-hmp-delay 8.0 \
		--expect "[net] virtio-net ready driver=virtio-net" \
		--expect "HTTP/" \
		--expect "[boot] desktop ready"

smoke-net-https: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--net \
		--usb \
		--seconds 90 \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "https example.com\n" \
		--post-hmp-delay 20.0 \
		--expect "[net] virtio-net ready driver=virtio-net" \
		--expect "[tls-ok] https example.com/ via" \
		--expect "verified_root=AAA Certificate Services" \
		--expect "[boot] desktop ready"

smoke-net-https-negative: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 30 \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "tlscheck\n" \
		--post-hmp-delay 2.0 \
		--expect "TLS hostname negative ok" \
		--expect "[boot] desktop ready"

smoke-net-browser-https: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--net \
		--usb \
		--seconds 100 \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser https://example.com/\n" \
		--post-hmp-delay 24.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/browser-https-smoke.ppm" \
		--expect-framebuffer-window \
		--expect "[net] virtio-net ready driver=virtio-net" \
		--expect "[tls-ok] https example.com/ via" \
		--expect "verified_root=AAA Certificate Services" \
		--expect "[boot] desktop ready"

smoke-net-browser-google: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--net \
		--usb \
		--seconds 140 \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser https://google.com/\n" \
		--post-hmp-delay 36.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/browser-google-smoke.ppm" \
		--expect-framebuffer-window \
		--expect "[net] virtio-net ready driver=virtio-net" \
		--expect "[tls-ok] https google.com/ via" \
		--expect "verified_root=GTS Root R1" \
		--expect "[boot] desktop ready"

smoke-usb-init: build-usb-init
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(USB_INIT_BIOS)" \
		--fsimg "$(USB_INIT_FSIMG)" \
		--usb \
		--seconds $(SMOKE_USB_SECONDS) \
		--expect "[xhci] active init ready" \
		--expect "[input] USB keyboard detected; PS/2 keyboard fallback disabled" \
		--expect "[input] USB mouse detected; PS/2 mouse fallback disabled" \
		--expect "[ring3 pid=1] sentinel ok" \
		--expect "[ring3 pid=2] sentinel ok" \
		--expect "[boot] desktop ready"

smoke-hotplug-usb-init: build-usb-init
	python3 $(CURDIR)/scripts/qemu_hotplug_smoke.py \
		--bios "$(USB_INIT_BIOS)" \
		--fsimg "$(USB_INIT_FSIMG)"

smoke-kernel-units: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_SECONDS) \
		--expect "[selftest] kernel unit checks ok=17 fail=0" \
		--expect "[boot] desktop ready"

smoke-boot-budget: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_BOOT_BUDGET_SECONDS) \
		--expect "[boot] desktop ready"

smoke-lowmem: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--memory 256M \
		--seconds $(SMOKE_SECONDS) \
		--expect "[boot] desktop ready"

smoke-smp2: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--smp 2 \
		--seconds $(SMOKE_SECONDS) \
		--expect "[boot] desktop ready"

smoke-vga-cirrus: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--vga cirrus \
		--seconds $(SMOKE_VGA_SECONDS) \
		--expect "[boot] desktop ready"

build:
	cargo build --release --target $(TARGET) \
		-Z build-std=core,compiler_builtins,alloc \
		-Z build-std-features=compiler-builtins-mem
	RUSTFLAGS="-C link-arg=-T$(CURDIR)/userspace/hello/linker.ld" \
		cargo build --manifest-path $(CURDIR)/userspace/hello/Cargo.toml \
		--release \
		--target $(TARGET) \
		--target-dir $(CURDIR)/target/userspace/hello \
		-Z build-std=core,compiler_builtins
	(cd disk-image && cargo run --bin disk-image -- "$(KERNEL)")
	(cd disk-image && cargo run --bin fs-image -- "$(FSIMG)" "$(USER_TARGET)" "$(USER_EXEC_TARGET)" "$(USER_PIPE_TARGET)" "$(USER_READ_TARGET)" "$(USER_PIPERD_TARGET)" "$(USER_PIPEWR_TARGET)" "$(USER_KEYECHO_TARGET)" "$(USER_TERMINAL_TARGET)" "$(USER_NETDEMO_TARGET)" "$(USER_WGET_TARGET)" "$(USER_SDKDEMO_TARGET)" "$(USER_GUIDEMO_TARGET)" "$(USER_NOTES_TARGET)" "$(USER_EDITOR_TARGET)" "$(USER_TRASH_TARGET)" "$(USER_SCREENSHOT_TARGET)")

build-usb-init: build

clean:
	cargo clean
	rm -rf target
	rm -rf disk-image/target
