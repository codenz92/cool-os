.PHONY: run run-net run-usb run-usb-init run-smooth run-remote run-remote-net run-vnc run-vnc-net run-headless run-headless-net run-headless-usb run-headless-usb-init smoke smoke-ui smoke-login-screen smoke-lock-screen smoke-ui-ready-state smoke-framebuffer smoke-ui-goldens smoke-browser-png smoke-browser-html smoke-ui-settings smoke-ui-visual-assertions smoke-start-menu smoke-userspace-sdk smoke-userspace-gui smoke-userspace-utils smoke-userspace-file-open smoke-package-app smoke-coolfs-root smoke-coolfs-native smoke-phase28-permissions smoke-phase29-sessions smoke-phase31-accounts smoke-phase32-isolation smoke-phase33-process-control smoke-phase34-tty-jobs smoke-phase35-tty-input smoke-phase36-userspace-shell smoke-phase37-coreutils smoke-phase38-apps smoke-phase39-recovery smoke-phase40-shell-semantics smoke-phase41-fs-durability smoke-phase42-app-consistency smoke-phase43-observability smoke-phase44-devkit smoke-phase45-smoothness smoke-phase46-adaptive-refresh smoke-phase47-evented-userspace smoke-phase48-terminal-tui smoke-phase49-browser-engine smoke-phase50-css-layout smoke-phase51-browser-forms smoke-phase52-dom-events smoke-phase53-dom-forms smoke-phase54-browser-post smoke-phase55-browser-session smoke-phase56-css-box-model smoke-net-api smoke-net-wget smoke-net-https smoke-net-https-negative smoke-net-browser-https smoke-net-browser-google smoke-usb-init smoke-hotplug-usb-init smoke-kernel-units smoke-boot-budget smoke-lowmem smoke-smp2 smoke-vga-cirrus build build-usb-init clean

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
USER_TTYREAD_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/ttyread
USER_SH_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/sh
USER_LS_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/ls
USER_CAT_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/cat
USER_ECHO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/echo
USER_PWD_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/pwd
USER_MKDIR_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/mkdir
USER_TOUCH_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/touch
USER_RM_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/rm
USER_WRITEFILE_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/writefile
USER_CP_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/cp
USER_MV_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/mv
USER_GREP_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/grep
USER_HEAD_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/head
USER_TAIL_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/tail
USER_DATE_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/date
USER_UNAME_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/uname
USER_CLEAR_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/clear
USER_STAT_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/stat
USER_SYNC_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/sync
USER_DEVKIT_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/devkit
USER_POLLDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/polldemo
USER_TUIDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/tuidemo
USER_NETDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/netdemo
USER_WGET_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/wget
USER_SDKDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/sdkdemo
USER_GUIDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/guidemo
USER_NOTES_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/notes
USER_EDITOR_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/editor
USER_TRASH_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/trash
USER_SCREENSHOT_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/screenshot
USER_PROCDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/procdemo
USER_PROCSLEEP_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/procsleep
USER_SENTINEL_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/sentinel
USER_BADPTR_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/badptr
USER_BADWRITE_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/badwrite
USER_BADMMAP_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/badmmap
USER_BADEXEC_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/badexec
USER_BADUSERREAD_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/baduserread
SMOKE_SECONDS ?= 18
SMOKE_FRAMEBUFFER_SECONDS ?= 30
SMOKE_INTERACTIVE_SECONDS ?= $(SMOKE_FRAMEBUFFER_SECONDS)
SMOKE_PRE_TYPE_DELAY ?= 1.5
SMOKE_TYPE_KEY_DELAY ?= 0.15
SMOKE_RETRIES ?= 1
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

run-smooth: build-usb-init
	@echo "Booting coolOS with phase 46 adaptive high-refresh defaults..."
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

smoke-login-screen: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/login-screen.ppm" \
		--expect-framebuffer-login \
		--expect "[boot] login ready" \
		--expect "[boot] desktop ready"

smoke-lock-screen: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--fw-cmd "lock" \
		--interact-after "session locked" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/lock-screen.ppm" \
		--expect-framebuffer-login \
		--expect "session locked" \
		--expect "[session] locked" \
		--expect "[boot] desktop ready"

smoke-ui-ready-state: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-esc" \
		--post-hmp-delay 0.8 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-ready-state.ppm" \
		--expect-framebuffer-start-menu \
		--expect "[boot] desktop ready" \
		--expect "[ui] ready pinned=Terminal|File Manager|System Monitor|Diagnostics|Display Settings|Accounts|Personalize"

smoke-framebuffer: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
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
		--expect "[browser] open file:///tmp/pngtest.png" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
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
		--expect "[browser] open file:///tmp/phase19.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-ui-goldens: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "ui-golden-desktop" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-golden-desktop.ppm" \
		--expect-framebuffer-desktop \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "ui-golden-file-manager" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
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
		--usb \
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
		--usb \
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
		--usb \
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
		--usb \
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
		--expect "sdkdemo: libcool sdk=1 abi=10" \
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
		--seconds 60 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/notes /documents/notes.txt smoke" \
		--expect "notes: window opened" \
		--expect "notes: saved" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-editor" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 60 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/editor /documents/editor.txt smoke" \
		--expect "editor: window opened" \
		--expect "editor: saved" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-trash" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/trash smoke" \
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
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/screenshot smoke" \
		--expect "screenshot: window opened" \
		--expect "screenshot: queued /Pictures/SMOKE.PPM" \
		--expect "[boot] desktop ready"

smoke-userspace-file-open: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/editor /documents/phase23.txt smoke" \
		--expect "editor: window opened" \
		--expect "editor: saved /documents/phase23.txt" \
		--expect "[boot] desktop ready"

smoke-package-app: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "pkg install /Packages/guidemo.pkg;;pkg run pkgdemo;;pkg remove pkgdemo" \
		--expect "[pkg] installed app.phase25.guidemo name=Packaged GUI Demo exec=/bin/guidemo" \
		--expect "[pkg] launched app.phase25.guidemo exec=/bin/guidemo pid=" \
		--expect "guidemo: window opened" \
		--expect "guidemo: presented frame" \
	--expect "[pkg] removed app.phase25.guidemo" \
	--expect "[boot] desktop ready"

smoke-coolfs-root: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "vfs;;path /;;path /FAT;;path /bin/hello.txt;;df;;fsck" \
		--expect "/ type=coolfs flags=rw,native-root,normalized-paths,uid-gid-mode" \
		--expect "/FAT type=fat32 flags=rw,legacy-import,optional" \
		--expect "/ kind=dir mount=coolfs size=448" \
		--expect "/FAT kind=dir mount=fat32 size=0" \
		--expect "/bin/hello.txt kind=file mount=coolfs size=27" \
		--expect "coolfs root ok" \
		--expect "[boot] desktop ready"

smoke-phase28-permissions: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "whoami;;perm /bin/hello;;perm /TMP;;write /TMP/P28 ok;;chmod 400 /TMP/P28;;write /TMP/P28 no;;chmod 600 /TMP/P28;;write /TMP/P28 yes;;hash /TMP/P28;;perm /TMP/P28;;exec /TMP/P28" \
		--expect "root uid=1000 gid=1000 caps=all" \
		--expect "/bin/hello file uid=0 gid=0 mode=755" \
		--expect "/TMP dir uid=1000 gid=1000 mode=777" \
		--expect "wrote /TMP/P28" \
		--expect "chmod /TMP/P28" \
		--expect "write: permission denied" \
		--expect "hash /TMP/P28 len=3 sum=337" \
		--expect "/TMP/P28 file uid=1000 gid=1000 mode=600" \
		--expect "exec: permission denied" \
		--expect "[boot] desktop ready"

smoke-phase29-sessions: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "id;;login guest guest;;whoami;;write /Users/guest/P29 ok;;chown 0:0 /Users/guest/P29;;login root cool;;chown 0:0 /Users/guest/P29;;perm /Users/guest/P29;;services fail package-db;;services run;;services package-db" \
		--expect "root uid=1000 gid=1000 role=admin home=/Users/root" \
		--expect "session user guest uid=1001" \
		--expect "guest uid=1001 gid=1000 caps=read-fs,write-fs,exec,network,desktop" \
		--expect "wrote /Users/guest/P29" \
		--expect "chown: permission denied" \
		--expect "session user root uid=1000" \
		--expect "/Users/guest/P29 file uid=0 gid=0 mode=644" \
		--expect "service supervisor tick" \
		--expect "package-db state=running restart=on-failure uid=200 gid=200" \
		--expect "[boot] desktop ready"

smoke-phase31-accounts: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(SMOKE_ARTIFACT_DIR)/phase31-accounts.img"
	cp "$(FSIMG)" "$(SMOKE_ARTIFACT_DIR)/phase31-accounts.img"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-auth" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "account add p31 p31pass31 admin;;id p31;;account disable guest;;login guest guest;;account enable guest;;account pass p31 p31next31;;login p31 p31next31;;whoami" \
		--expect "account added p31 uid=" \
		--expect "p31 uid=" \
		--expect "account disabled guest uid=1001 role=user login=disabled" \
		--expect "login: login disabled" \
		--expect "account enabled guest uid=1001 role=user login=enabled" \
		--expect "account password p31" \
		--expect "session user p31 uid=" \
		--expect "p31 uid=1002 gid=1000 caps=all home=/Users/p31" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-role-delete" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "account add p31 p31pass31 admin;;account role p31 user;;id p31;;account role p31 admin;;account delete p31" \
		--expect "account added p31 uid=" \
		--expect "account role p31 uid=1002 role=user login=enabled" \
		--expect "p31 uid=1002 gid=1000 role=user home=/Users/p31 login=enabled" \
		--expect "account role p31 uid=1002 role=admin login=enabled" \
		--expect "account deleted p31" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-throttle" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "login guest badpass31;;login guest badpass31;;login guest badpass31;;login guest guest" \
		--expect "login: bad password" \
		--expect "login: login temporarily locked" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-setup" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase31-accounts.img" \
		--fs-writable \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "setup owner31 ownerpass31;;cat /CONFIG/USERS.DB;;flush;;id owner31;;login root cool" \
		--expect "first-run admin owner31 uid=" \
		--expect "owner31:1002:1000:admin:/Users/owner31" \
		--expect "flush: ok" \
		--expect "owner31 uid=" \
		--expect "login: login disabled" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-persist" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase31-accounts.img" \
		--fs-writable \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "login owner31 ownerpass31;;cat /CONFIG/USERS.DB;;id owner31;;login root cool" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[session] login owner31 uid=" \
		--expect "owner31:1002:1000:admin:/Users/owner31" \
		--expect "owner31 uid=" \
		--expect "login: login disabled" \
		--expect "[boot] desktop ready"

smoke-phase32-isolation: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "job run /bin/badptr;;job run /bin/badwrite;;job run /bin/badmmap;;job run /bin/badexec;;exec /bin/baduserread;;crash" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "badptr: denied" \
		--expect "badwrite: denied" \
		--expect "badmmap: denied" \
		--expect "badexec: denied" \
		--expect "baduserread: touching kernel page" \
		--expect "reason=user page fault" \
		--expect "[boot] desktop ready"

smoke-phase33-process-control: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-procdemo" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 30 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/procdemo" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "procdemo: child pgid" \
		--expect "procdemo: usr1 ok" \
		--expect "procdemo: stop ok" \
		--expect "procdemo: cont ok" \
		--expect "procdemo: group term count=1" \
		--expect "procdemo: wait exit 143" \
		--expect "procdemo: phase33 ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-jobs" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 30 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "job run /bin/procsleep;;jobs;;job pause last;;jobs;;job resume last;;job cancel last;;jobs" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "job #" \
		--expect "pid=" \
		--expect "paused" \
		--expect "cancelled" \
		--expect "[boot] desktop ready"

smoke-phase34-tty-jobs: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-foreground" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 35 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/procsleep short;;tty" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "foreground /bin/procsleep" \
		--expect "procsleep: pid=" \
		--expect "procsleep: done" \
		--expect "[fg done] /bin/procsleep" \
		--expect "tty #" \
		--expect "foreground pgid=-" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-background" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 35 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "job run /bin/procsleep;;tty;;jobs;;job pause last;;jobs;;bg last;;job cancel last;;jobs" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "job #" \
		--expect "tty #" \
		--expect "foreground pgid=-" \
		--expect "paused" \
		--expect "background job #" \
		--expect "cancelled" \
		--expect "[boot] desktop ready"

smoke-phase35-tty-input: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/ttyread" \
		--no-auto-login \
		--interact-after "ttyread: ready" \
		--type-text "hello from tty\n" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "foreground /bin/ttyread" \
		--expect "ttyread: ready" \
		--expect "ttyread: got hello from tty" \
		--expect "[fg done] /bin/ttyread" \
		--expect "[boot] desktop ready"

smoke-phase36-userspace-shell: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 50 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "sh" \
		--no-auto-login \
		--interact-after "sh: ready abi=10" \
		--type-text "pwd\nls /bin\ncat /bin/hello.txt\necho userspace shell ok\nexit\n" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "foreground /bin/sh" \
		--expect "sh: ready abi=10" \
		--expect "Hello from /bin/hello.txt!" \
		--expect "userspace shell ok" \
		--expect "[fg done] /bin/sh pid=3 exit=0" \
		--expect "[boot] desktop ready"

smoke-phase37-coreutils: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 55 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "sh" \
		--no-auto-login \
		--interact-after "sh: ready abi=10" \
		--type-text "run /bin/pwd\nrun /bin/echo external coreutils ok\nrun /bin/ls /bin\nrun /bin/cat /bin/hello.txt\nrun /bin/mkdir /TMP/PH37\nrun /bin/writefile /TMP/PH37/NOTE coreutils file ok\nrun /bin/cat /TMP/PH37/NOTE\nrun /bin/touch /TMP/PH37/EMPTY\nrun /bin/rm /TMP/PH37/EMPTY\nrun /bin/rm /TMP/PH37\nexit\n" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "foreground /bin/sh" \
		--expect "sh: ready abi=10" \
		--expect "external coreutils ok" \
		--expect "F	ls" \
		--expect "Hello from /bin/hello.txt!" \
		--expect "coreutils file ok" \
		--expect "[fg done] /bin/sh pid=3 exit=0" \
		--expect "[boot] desktop ready"

smoke-phase38-apps: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-editor" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 60 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/editor /documents/phase38-editor.txt smoke" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "editor: window opened" \
		--expect "editor: saved /documents/phase38-editor.txt" \
		--expect "editor: verified /documents/phase38-editor.txt" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-trash" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/trash smoke" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "trash: window opened" \
		--expect "trash: listed" \
		--expect "trash: empty ok" \
		--expect "trash: verified empty" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-screenshot" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/screenshot smoke" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "screenshot: window opened" \
		--expect "screenshot: queued /Pictures/SMOKE.PPM" \
		--expect "screenshot: saved /Pictures/SMOKE.PPM" \
		--expect "[boot] desktop ready"

smoke-phase39-recovery: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "recovery;;recovery repair;;cat /RECOVERY/LAST-REPAIR.TXT;;recovery fsck-on-boot on;;recovery" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "RECOVERY" \
		--expect "mode=normal recovery=available" \
		--expect "boot=BIOS/VBE root=/ type=coolfs" \
		--expect "wrote /RECOVERY/LAST-REPAIR.TXT" \
		--expect "coolOS recovery repair report" \
		--expect "storage.fsck_on_boot=true saved" \
		--expect "fsck_on_boot=true" \
		--expect "[boot] desktop ready"

smoke-phase40-shell-semantics: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 75 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "sh" \
		--no-auto-login \
		--interact-after "sh: ready abi=10" \
		--type-key-delay $(SMOKE_TYPE_KEY_DELAY) \
		--type-text "cd /TMP\npwd\necho phase40 redirect ok > p40.txt\ncat p40.txt\ncat p40.txt | grep redirect\ncp p40.txt p40-copy.txt\nmv p40-copy.txt p40-moved.txt\nstat p40-moved.txt\nuname\ndate\nsync\nexit\n" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "sh: ready abi=10" \
		--expect "/tmp" \
		--expect "phase40 redirect ok" \
		--expect "kind=file size=" \
		--expect "coolOS coolOS-userspace-abi/10 x86_64" \
		--expect "sync: ok" \
		--expect "[fg done] /bin/sh pid=3 exit=0" \
		--expect "[boot] desktop ready"

smoke-phase41-fs-durability: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(SMOKE_ARTIFACT_DIR)/phase41.img"
	cp "$(FSIMG)" "$(SMOKE_ARTIFACT_DIR)/phase41.img"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-write" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase41.img" \
		--fs-writable \
		--usb \
		--seconds 70 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "sh" \
		--no-auto-login \
		--interact-after "sh: ready abi=10" \
		--type-key-delay $(SMOKE_TYPE_KEY_DELAY) \
		--type-text "echo phase41 durable ok > /TMP/P41.TXT\nsync\nexit\n" \
		--expect "sync: ok" \
		--expect "[fg done] /bin/sh pid=3 exit=0" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-remount" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase41.img" \
		--fs-writable \
		--usb \
		--seconds 70 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "sh" \
		--no-auto-login \
		--interact-after "sh: ready abi=10" \
		--type-key-delay $(SMOKE_TYPE_KEY_DELAY) \
		--type-text "cat /TMP/P41.TXT\nstat /TMP/P41.TXT\nexit\n" \
		--expect "phase41 durable ok" \
		--expect "kind=file size=" \
		--expect "[fg done] /bin/sh pid=3 exit=0" \
		--expect "[boot] desktop ready"

smoke-phase42-app-consistency: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "diagnostics;;logs;;profiler;;devkit" \
		--expect "DIAGNOSTICS" \
		--expect "LOG VIEW" \
		--expect "BOOT/SESSION PROFILER" \
		--expect "DEVKIT" \
		--expect "coolOS devkit ABI=10" \
		--expect "[boot] desktop ready"

smoke-phase43-observability: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "sysreport;;sysreport write;;cat /LOGS/SYSREPORT.TXT" \
		--expect "SYSREPORT" \
		--expect "== kernel log ==" \
		--expect "wrote /LOGS/SYSREPORT.TXT" \
		--expect "coolOS system report" \
		--expect "== services ==" \
		--expect "[boot] desktop ready"

smoke-phase44-devkit: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "devkit;;cat /SDK/README.TXT;;exec /bin/devkit" \
		--expect "coolOS devkit ABI=10" \
		--expect "coolOS SDK" \
		--expect "ABI version: 10" \
		--expect "foreground /bin/devkit" \
		--expect "template: /SDK/APP_TEMPLATE.RS" \
		--expect "[fg done] /bin/devkit" \
		--expect "[boot] desktop ready"

smoke-phase45-smoothness: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 35 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "compositor" \
		--expect "COMPOSITOR" \
		--expect "frame_source full=" \
		--expect "cursor_mode=overlay" \
		--expect "passive_frame_hz=36" \
		--expect "[boot] desktop ready"

smoke-phase46-adaptive-refresh: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 35 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "smoothness" \
		--expect "COMPOSITOR" \
		--expect "frame_pacing mode=" \
		--expect "target_hz=" \
		--expect "idle_hz=36" \
		--expect "active_hz=144" \
		--expect "boost_ms=750" \
		--expect "frame_budget target_ticks=" \
		--expect "cursor_mode=overlay" \
		--expect "[boot] desktop ready"

smoke-phase47-evented-userspace: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/polldemo" \
		--expect "polldemo: abi=10" \
		--expect "polldemo: timeout ok" \
		--expect "polldemo: pipe ok" \
		--expect "polldemo: child ok" \
		--expect "polldemo: done" \
		--expect "[boot] desktop ready"

smoke-phase48-terminal-tui: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 60 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "exec /bin/tuidemo" \
		--no-auto-login \
		--interact-after "tuidemo: raw ready" \
		--type-key-delay $(SMOKE_TYPE_KEY_DELAY) \
		--type-text "q" \
		--expect "tuidemo: abi=10" \
		--expect "tuidemo: raw ready" \
		--expect "press q to exit without Enter" \
		--expect "tuidemo: raw exit key=q" \
		--expect "tuidemo: done" \
		--expect "[fg done] /bin/tuidemo pid=3 exit=0" \
		--expect "[boot] desktop ready"

smoke-phase49-browser-engine: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE49.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase49-browser-engine.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase49.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase50-css-layout: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE50.CSS.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase50-css-layout.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase50.css.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase51-browser-forms: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE51.FORM.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase51-browser-forms.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase51.form.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase52-dom-events: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE52.DOM.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase52-dom-events.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase52.dom.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase53-dom-forms: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE53.DOM.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase53-dom-forms.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase53.dom.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase54-browser-post: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--net \
		--usb \
		--seconds 100 \
		--no-auto-login \
		--fw-cmd "browser file:///tmp/phase54.post.html" \
		--interact-after "[browser] open file:///tmp/phase54.post.html" \
		--hmp "sendkey tab" \
		--hmp "sendkey ret" \
		--post-hmp-delay 28.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase54-browser-post.ppm" \
		--expect-framebuffer-window \
		--expect "[net] virtio-net ready driver=virtio-net" \
		--expect "[browser] open file:///tmp/phase54.post.html" \
		--expect "[http] POST https://example.com/post body=" \
		--expect "[tls-ok] https example.com/post via" \
		--expect "verified_root=AAA Certificate Services" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase55-browser-session: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser browser://session\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase55-browser-session.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open browser://session" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase56-css-box-model: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE56.BOX.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase56-css-box-model.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase56.box.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-coolfs-native: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(SMOKE_ARTIFACT_DIR)/coolfs-native.img"
	cp "$(FSIMG)" "$(SMOKE_ARTIFACT_DIR)/coolfs-native.img"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-write" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/coolfs-native.img" \
		--fs-writable \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "write /TMP/PHASE27.TXT native coolfs persists;;flush;;path /TMP/PHASE27.TXT;;hash /TMP/PHASE27.TXT" \
		--expect "wrote /TMP/PHASE27.TXT" \
		--expect "flush: ok" \
		--expect "/TMP/PHASE27.TXT kind=file mount=coolfs size=22" \
		--expect "hash /TMP/PHASE27.TXT len=22 sum=2250" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-delete" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/coolfs-native.img" \
		--fs-writable \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "path /TMP/PHASE27.TXT;;hash /TMP/PHASE27.TXT;;rm /TMP/PHASE27.TXT;;flush;;path /TMP/PHASE27.TXT" \
		--expect "/TMP/PHASE27.TXT kind=file mount=coolfs size=22" \
		--expect "hash /TMP/PHASE27.TXT len=22 sum=2250" \
		--expect "removed /TMP/PHASE27.TXT" \
		--expect "flush: ok" \
		--expect "/TMP/PHASE27.TXT kind=missing mount=coolfs size=0" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-remount" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/coolfs-native.img" \
		--fs-writable \
		--usb \
		--seconds 45 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "path /TMP/PHASE27.TXT;;fsck" \
		--expect "/TMP/PHASE27.TXT kind=missing mount=coolfs size=0" \
		--expect "coolfs root ok" \
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
		--expect "[browser] open https://example.com/" \
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
		--expect "[browser] open https://google.com/" \
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
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
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
	(cd disk-image && cargo run --bin fs-image -- "$(FSIMG)" "$(USER_TARGET)" "$(USER_EXEC_TARGET)" "$(USER_PIPE_TARGET)" "$(USER_READ_TARGET)" "$(USER_PIPERD_TARGET)" "$(USER_PIPEWR_TARGET)" "$(USER_KEYECHO_TARGET)" "$(USER_TERMINAL_TARGET)" "$(USER_TTYREAD_TARGET)" "$(USER_NETDEMO_TARGET)" "$(USER_WGET_TARGET)" "$(USER_SDKDEMO_TARGET)" "$(USER_GUIDEMO_TARGET)" "$(USER_NOTES_TARGET)" "$(USER_EDITOR_TARGET)" "$(USER_TRASH_TARGET)" "$(USER_SCREENSHOT_TARGET)" "$(USER_PROCDEMO_TARGET)" "$(USER_PROCSLEEP_TARGET)" "$(USER_SENTINEL_TARGET)" "$(USER_BADPTR_TARGET)" "$(USER_BADWRITE_TARGET)" "$(USER_BADMMAP_TARGET)" "$(USER_BADEXEC_TARGET)" "$(USER_BADUSERREAD_TARGET)" "$(USER_SH_TARGET)" "$(USER_LS_TARGET)" "$(USER_CAT_TARGET)" "$(USER_ECHO_TARGET)" "$(USER_PWD_TARGET)" "$(USER_MKDIR_TARGET)" "$(USER_TOUCH_TARGET)" "$(USER_RM_TARGET)" "$(USER_WRITEFILE_TARGET)" "$(USER_CP_TARGET)" "$(USER_MV_TARGET)" "$(USER_GREP_TARGET)" "$(USER_HEAD_TARGET)" "$(USER_TAIL_TARGET)" "$(USER_DATE_TARGET)" "$(USER_UNAME_TARGET)" "$(USER_CLEAR_TARGET)" "$(USER_STAT_TARGET)" "$(USER_SYNC_TARGET)" "$(USER_DEVKIT_TARGET)" "$(USER_POLLDEMO_TARGET)" "$(USER_TUIDEMO_TARGET)")

build-usb-init: build

clean:
	cargo clean
	rm -rf target
	rm -rf disk-image/target
