.PHONY: run run-uefi run-uefi-secure run-uefi-ahci run-uefi-nvme run-uefi-usb-storage run-uefi-usb-storage-secure run-uefi-usb-storage-safe run-physical-installer-sim run-installer run-uefi-installer run-uefi-ahci-installer run-uefi-nvme-installer run-installed run-uefi-installed run-uefi-ahci-installed run-uefi-nvme-installed run-net run-usb run-usb-init run-smooth run-remote run-remote-net run-vnc run-vnc-net run-headless run-headless-net run-headless-usb run-headless-usb-init smoke smoke-ui smoke-login-screen smoke-lock-screen smoke-ui-ready-state smoke-framebuffer smoke-ui-goldens smoke-browser-png smoke-browser-html smoke-ui-settings smoke-ui-visual-assertions smoke-start-menu smoke-userspace-sdk smoke-userspace-gui smoke-userspace-utils smoke-userspace-file-open smoke-package-app smoke-coolfs-root smoke-coolfs-native smoke-phase28-permissions smoke-phase29-sessions smoke-phase31-accounts smoke-phase32-isolation smoke-phase33-process-control smoke-phase34-tty-jobs smoke-phase35-tty-input smoke-phase36-userspace-shell smoke-phase37-coreutils smoke-phase38-apps smoke-phase39-recovery smoke-phase40-shell-semantics smoke-phase41-fs-durability smoke-phase42-app-consistency smoke-phase43-observability smoke-phase44-devkit smoke-phase45-smoothness smoke-phase46-adaptive-refresh smoke-pointer-tablet smoke-phase47-evented-userspace smoke-phase48-terminal-tui smoke-phase49-browser-engine smoke-phase50-css-layout smoke-phase51-browser-forms smoke-phase52-dom-events smoke-phase53-dom-forms smoke-phase54-browser-post smoke-phase55-browser-session smoke-phase56-css-box-model smoke-phase57-browser-layout smoke-phase58-browser-subresources smoke-phase59-browser-js smoke-phase60-browser-webapi smoke-phase61-browser-compat smoke-phase62-resource-limits smoke-phase63-memory-pressure smoke-phase64-services smoke-phase65-update-rollback smoke-phase66-boot-health smoke-phase67-update-trust smoke-phase68-update-keys smoke-phase69-package-trust smoke-phase70-package-payloads smoke-phase71-browser-engine-port smoke-phase72-threads-futex smoke-phase73-tls-pthread smoke-phase74-pthread-libc smoke-phase75-dynlink smoke-phase76-dynlink-deps smoke-phase77-file-mmap smoke-phase80-firstboot reset-firstboot-smoke-image smoke-phase81-firstboot-recovery smoke-phase82-installer smoke-phase83-self-booting-installer smoke-phase84-installer-v2 smoke-phase85-uefi-gpt smoke-phase86-ahci-storage smoke-phase87-usb-storage-root smoke-phase88-nvme-storage smoke-phase89-baremetal-readiness smoke-phase90-physical-installer smoke-phase91-hardware-readiness smoke-phase92-secure-boot smoke-phase93-secure-boot smoke-net-api smoke-net-wget smoke-net-https smoke-net-https-negative smoke-net-browser-https smoke-net-browser-google smoke-usb-init smoke-hotplug-usb-init smoke-kernel-units smoke-boot-budget smoke-lowmem smoke-smp2 smoke-vga-cirrus build build-uefi build-uefi-safe build-secure-boot-keys build-uefi-secure build-uefi-secure-loader-tamper build-uefi-secure-kernel-tamper build-usb-image build-usb-safe-image build-usb-secure-image build-usb-secure-loader-tamper-image build-usb-secure-kernel-tamper-image verify-secure-boot-artifacts tamper-secure-boot-artifacts build-usb-init clean

TARGET  := x86_64-unknown-none.json
KERNEL  := $(CURDIR)/target/x86_64-unknown-none/release/cool_os
BIOS    := $(CURDIR)/target/x86_64-unknown-none/release/bios.img
UEFI    := $(CURDIR)/target/x86_64-unknown-none/release/uefi.img
UEFI_SAFE := $(CURDIR)/target/x86_64-unknown-none/release/uefi-safe.img
UEFI_SECURE := $(CURDIR)/target/x86_64-unknown-none/release/uefi-secure.img
UEFI_SECURE_LOADER_TAMPER := $(CURDIR)/target/x86_64-unknown-none/release/uefi-secure-loader-tamper.img
UEFI_SECURE_KERNEL_TAMPER := $(CURDIR)/target/x86_64-unknown-none/release/uefi-secure-kernel-tamper.img
FSIMG   := $(CURDIR)/target/x86_64-unknown-none/release/fs.img
USB_IMAGE := $(CURDIR)/target/x86_64-unknown-none/release/coolos-usb.img
USB_SAFE_IMAGE := $(CURDIR)/target/x86_64-unknown-none/release/coolos-usb-safe.img
USB_SECURE_IMAGE := $(CURDIR)/target/x86_64-unknown-none/release/coolos-usb-secure.img
USB_SECURE_LOADER_TAMPER_IMAGE := $(CURDIR)/target/x86_64-unknown-none/release/coolos-usb-secure-loader-tamper.img
USB_SECURE_KERNEL_TAMPER_IMAGE := $(CURDIR)/target/x86_64-unknown-none/release/coolos-usb-secure-kernel-tamper.img
SECURE_BOOT_DIR ?= $(CURDIR)/target/secure-boot
SECURE_BOOT_PYDEPS ?= $(SECURE_BOOT_DIR)/pydeps
USB_IMAGE_SIZE_MIB ?= 96
USB_SAFE_FB_WIDTH ?= 1024
USB_SAFE_FB_HEIGHT ?= 768
USB_INIT_BIOS := $(BIOS)
USB_INIT_FSIMG := $(FSIMG)
QEMU_CPU ?= max
QEMU_RTC ?= -rtc base=utc,clock=host
QEMU_VNC ?= 127.0.0.1:1
QEMU_DISPLAY ?= cocoa,zoom-to-fit=on
QEMU_EFI_CODE ?= $(shell if [ -f /opt/homebrew/share/qemu/edk2-x86_64-code.fd ]; then echo /opt/homebrew/share/qemu/edk2-x86_64-code.fd; elif [ -f /usr/local/share/qemu/edk2-x86_64-code.fd ]; then echo /usr/local/share/qemu/edk2-x86_64-code.fd; else echo edk2-x86_64-code.fd; fi)
QEMU_UEFI := -drive if=pflash,format=raw,readonly=on,file="$(QEMU_EFI_CODE)"
QEMU_EFI_SECURE_CODE ?= $(shell if [ -f /opt/homebrew/share/qemu/edk2-x86_64-secure-code.fd ]; then echo /opt/homebrew/share/qemu/edk2-x86_64-secure-code.fd; elif [ -f /usr/local/share/qemu/edk2-x86_64-secure-code.fd ]; then echo /usr/local/share/qemu/edk2-x86_64-secure-code.fd; else echo edk2-x86_64-secure-code.fd; fi)
QEMU_EFI_VARS_TEMPLATE ?= $(shell if [ -f /opt/homebrew/share/qemu/edk2-i386-vars.fd ]; then echo /opt/homebrew/share/qemu/edk2-i386-vars.fd; elif [ -f /usr/local/share/qemu/edk2-i386-vars.fd ]; then echo /usr/local/share/qemu/edk2-i386-vars.fd; else echo edk2-i386-vars.fd; fi)
QEMU_EFI_SECURE_VARS ?= $(SECURE_BOOT_DIR)/OVMF_VARS.secboot.fd
QEMU_SECURE_STATUS ?= mode=qemu-secure-fw loader=signed-pe kernel=verified vars=enrolled enforcement=on
QEMU_UEFI_SECURE := -machine q35,smm=on -global driver=cfi.pflash01,property=secure,value=on -drive if=pflash,format=raw,readonly=on,file="$(QEMU_EFI_SECURE_CODE)" -drive if=pflash,format=raw,file="$(QEMU_EFI_SECURE_VARS)"
QEMU_POINTER ?= tablet
ifeq ($(QEMU_POINTER),mouse)
QEMU_POINTER_DEVICE := usb-mouse,bus=xhci.0
else ifeq ($(QEMU_POINTER),tablet)
QEMU_POINTER_DEVICE := usb-tablet,bus=xhci.0
else
$(error QEMU_POINTER must be either tablet or mouse)
endif
QEMU_USB_INPUT := -device qemu-xhci,id=xhci -device usb-kbd,bus=xhci.0 -device $(QEMU_POINTER_DEVICE)
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
USER_THREADDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/threaddemo
USER_TLSDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/tlsdemo
USER_PTHREADDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/pthreaddemo
USER_MMAPDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/mmapdemo
USER_LDDEMO_TARGET := $(CURDIR)/target/userspace/hello/x86_64-unknown-none/release/lddemo
PHASE75_DSO_TARGET := $(CURDIR)/target/phase75/libphase75.so
PHASE76_DEP_DSO_TARGET := $(CURDIR)/target/phase76/libphase76dep.so
PHASE76_MAIN_DSO_TARGET := $(CURDIR)/target/phase76/libphase76main.so
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
FIRSTBOOT_RESET_IMG ?= $(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img
FIRSTBOOT_RESET_LOGIN ?= ownerpass81\n
INSTALL_TARGET_IMG ?= $(SMOKE_ARTIFACT_DIR)/phase83-install-target.img
INSTALL_TARGET_SIZE ?= 96M
INSTALL_SMALL_TARGET_IMG ?= $(SMOKE_ARTIFACT_DIR)/phase84-small-target.img
INSTALL_SMALL_TARGET_SIZE ?= 32M
PHYSICAL_AHCI_TARGET_IMG ?= $(SMOKE_ARTIFACT_DIR)/phase90-physical-ahci-target.img
PHYSICAL_NVME_TARGET_IMG ?= $(SMOKE_ARTIFACT_DIR)/phase90-physical-nvme-target.img
PHASE91_UASP_IMG ?= $(SMOKE_ARTIFACT_DIR)/phase91-uasp-target.img

run: build
	@echo "Booting coolOS in QEMU with USB $(QEMU_POINTER) input..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi: build-uefi
	@echo "Booting coolOS in QEMU UEFI mode with USB $(QEMU_POINTER) input..."
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-drive format=raw,file="$(UEFI)",if=ide,index=0,snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-secure: build-uefi-secure
	@echo "Booting coolOS in QEMU Secure Boot test-key mode..."
	qemu-system-x86_64 \
		$(QEMU_UEFI_SECURE) \
		-device ich9-ahci,id=ahci \
		-drive if=none,id=securebootdisk,format=raw,file="$(UEFI_SECURE)",snapshot=on \
		-device ide-hd,drive=securebootdisk,bus=ahci.0 \
		-drive if=none,id=securerootdisk,file="$(FSIMG)",format=raw,snapshot=on \
		-device ide-hd,drive=securerootdisk,bus=ahci.1 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-fw_cfg name=opt/coolos/secure-boot,string="$(QEMU_SECURE_STATUS)" \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-ahci: build-uefi
	@echo "Booting coolOS in QEMU UEFI mode through AHCI/SATA..."
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-device ich9-ahci,id=ahci \
		-drive if=none,id=bootdisk,format=raw,file="$(UEFI)",snapshot=on \
		-device ide-hd,drive=bootdisk,bus=ahci.0 \
		-drive if=none,id=rootdisk,file="$(FSIMG)",format=raw,snapshot=on \
		-device ide-hd,drive=rootdisk,bus=ahci.1 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-nvme: build-usb-image
	@echo "Booting coolOS USB image through QEMU NVMe..."
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-drive if=none,id=nvmedisk,file="$(USB_IMAGE)",format=raw \
		-device nvme,drive=nvmedisk,serial=coolos-nvme0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-usb-storage: build-usb-image
	@echo "Booting coolOS USB image through QEMU xHCI mass storage..."
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-device qemu-xhci,id=xhci \
		-drive if=none,id=usbdisk,file="$(USB_IMAGE)",format=raw \
		-device usb-storage,drive=usbdisk,bus=xhci.0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device usb-kbd,bus=xhci.0 \
		-device $(QEMU_POINTER_DEVICE) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-usb-storage-secure: build-usb-secure-image
	@echo "Booting coolOS signed USB image through QEMU Secure Boot test-key mode..."
	qemu-system-x86_64 \
		$(QEMU_UEFI_SECURE) \
		-device qemu-xhci,id=xhci \
		-drive if=none,id=secureusbdisk,file="$(USB_SECURE_IMAGE)",format=raw \
		-device usb-storage,drive=secureusbdisk,bus=xhci.0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device usb-kbd,bus=xhci.0 \
		-device $(QEMU_POINTER_DEVICE) \
		-fw_cfg name=opt/coolos/secure-boot,string="$(QEMU_SECURE_STATUS)" \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-usb-storage-safe: build-usb-safe-image
	@echo "Booting coolOS safe USB image through QEMU xHCI mass storage..."
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-device qemu-xhci,id=xhci \
		-drive if=none,id=usbdisk,file="$(USB_SAFE_IMAGE)",format=raw \
		-device usb-storage,drive=usbdisk,bus=xhci.0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device usb-kbd,bus=xhci.0 \
		-device $(QEMU_POINTER_DEVICE) \
		-fw_cfg name=opt/coolos/safe-mode,string=1 \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-physical-installer-sim: build-usb-image
	@echo "Booting coolOS USB physical-installer simulation with AHCI and NVMe targets..."
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(PHYSICAL_AHCI_TARGET_IMG)" "$(PHYSICAL_NVME_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(PHYSICAL_AHCI_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(PHYSICAL_NVME_TARGET_IMG)"
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-device qemu-xhci,id=xhci \
		-drive if=none,id=usbdisk,file="$(USB_IMAGE)",format=raw,snapshot=on \
		-device usb-storage,drive=usbdisk,bus=xhci.0 \
		-device ich9-ahci,id=ahci \
		-drive if=none,id=ahcitarget,file="$(PHYSICAL_AHCI_TARGET_IMG)",format=raw \
		-device ide-hd,drive=ahcitarget,bus=ahci.0 \
		-drive if=none,id=nvmetarget,file="$(PHYSICAL_NVME_TARGET_IMG)",format=raw \
		-device nvme,drive=nvmetarget,serial=coolos-physical-nvme0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		-device usb-kbd,bus=xhci.0 \
		-device $(QEMU_POINTER_DEVICE) \
		-fw_cfg name=opt/coolos/installer,string=1 \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-installer: build
	@echo "Booting coolOS installer with writable self-boot target $(INSTALL_TARGET_IMG)..."
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-drive file="$(INSTALL_TARGET_IMG)",if=ide,format=raw,index=2 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-fw_cfg name=opt/coolos/installer,string=1 \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-installer: build-uefi
	@echo "Booting coolOS UEFI installer with writable GPT target $(INSTALL_TARGET_IMG)..."
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-drive format=raw,file="$(UEFI)",if=ide,index=0,snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-drive file="$(INSTALL_TARGET_IMG)",if=ide,format=raw,index=2 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-fw_cfg name=opt/coolos/installer,string=1 \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-ahci-installer: build-uefi
	@echo "Booting coolOS UEFI installer with AHCI/SATA target $(INSTALL_TARGET_IMG)..."
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-device ich9-ahci,id=ahci \
		-drive if=none,id=bootdisk,format=raw,file="$(UEFI)",snapshot=on \
		-device ide-hd,drive=bootdisk,bus=ahci.0 \
		-drive if=none,id=rootdisk,file="$(FSIMG)",format=raw,snapshot=on \
		-device ide-hd,drive=rootdisk,bus=ahci.1 \
		-drive if=none,id=targetdisk,file="$(INSTALL_TARGET_IMG)",format=raw \
		-device ide-hd,drive=targetdisk,bus=ahci.2 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-fw_cfg name=opt/coolos/installer,string=1 \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-nvme-installer: build-uefi
	@echo "Booting coolOS UEFI installer with NVMe target $(INSTALL_TARGET_IMG)..."
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-drive format=raw,file="$(UEFI)",if=ide,index=0,snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-drive if=none,id=targetdisk,file="$(INSTALL_TARGET_IMG)",format=raw \
		-device nvme,drive=targetdisk,serial=coolos-nvme0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-fw_cfg name=opt/coolos/installer,string=1 \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-installed: build
	@test -f "$(INSTALL_TARGET_IMG)" || (echo "Missing $(INSTALL_TARGET_IMG). Run make run-installer and install first." && exit 1)
	@echo "Booting installed coolOS target $(INSTALL_TARGET_IMG) as a standalone disk..."
	qemu-system-x86_64 \
		-drive file="$(INSTALL_TARGET_IMG)",if=ide,format=raw,index=0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-installed: build-uefi
	@test -f "$(INSTALL_TARGET_IMG)" || (echo "Missing $(INSTALL_TARGET_IMG). Run make run-uefi-installer and install first." && exit 1)
	@echo "Booting installed coolOS GPT target $(INSTALL_TARGET_IMG) under UEFI..."
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-drive file="$(INSTALL_TARGET_IMG)",if=ide,format=raw,index=0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-ahci-installed: build-uefi
	@test -f "$(INSTALL_TARGET_IMG)" || (echo "Missing $(INSTALL_TARGET_IMG). Run make run-uefi-ahci-installer and install first." && exit 1)
	@echo "Booting installed coolOS GPT target $(INSTALL_TARGET_IMG) under UEFI/AHCI..."
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-device ich9-ahci,id=ahci \
		-drive if=none,id=bootdisk,file="$(INSTALL_TARGET_IMG)",format=raw \
		-device ide-hd,drive=bootdisk,bus=ahci.0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-uefi-nvme-installed: build-uefi
	@test -f "$(INSTALL_TARGET_IMG)" || (echo "Missing $(INSTALL_TARGET_IMG). Run make run-uefi-nvme-installer and install first." && exit 1)
	@echo "Booting installed coolOS GPT target $(INSTALL_TARGET_IMG) under UEFI/NVMe..."
	qemu-system-x86_64 \
		$(QEMU_UEFI) \
		-drive if=none,id=bootdisk,file="$(INSTALL_TARGET_IMG)",format=raw \
		-device nvme,drive=bootdisk,serial=coolos-nvme0 \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-net: build
	@echo "Booting coolOS in QEMU with virtio-net and USB $(QEMU_POINTER) input..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-netdev user,id=net0 \
		-device virtio-net-pci,netdev=net0,disable-modern=on,disable-legacy=off \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-usb: build
	@echo "Booting coolOS in QEMU with xHCI-attached USB $(QEMU_POINTER) input..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)",snapshot=on \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-usb-init: build-usb-init
	@echo "Booting coolOS in QEMU with active xHCI init and USB $(QEMU_POINTER) input..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-smooth: build-usb-init
	@echo "Booting coolOS with phase 46 adaptive high-refresh defaults and USB $(QEMU_POINTER) input..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-vnc: build-usb-init
	@echo "Booting coolOS in QEMU VNC with USB $(QEMU_POINTER) input on $(QEMU_VNC)..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display vnc="$(QEMU_VNC)" \
		-debugcon stdio

run-vnc-net: build-usb-init
	@echo "Booting coolOS in QEMU VNC with virtio-net and USB $(QEMU_POINTER) input on $(QEMU_VNC)..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-netdev user,id=net0 \
		-device virtio-net-pci,netdev=net0,disable-modern=on,disable-legacy=off \
		-display vnc="$(QEMU_VNC)" \
		-debugcon stdio

run-remote: build-usb-init
	@echo "Booting coolOS in a QEMU window with USB $(QEMU_POINTER) input for remote desktop..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-display "$(QEMU_DISPLAY)" \
		-debugcon stdio

run-remote-net: build-usb-init
	@echo "Booting coolOS in a QEMU window with virtio-net and USB $(QEMU_POINTER) input..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(USB_INIT_BIOS)",snapshot=on \
		-drive file="$(USB_INIT_FSIMG)",if=ide,format=raw,index=1,snapshot=on \
		-m 512M \
		-cpu "$(QEMU_CPU)" \
		$(QEMU_RTC) \
		-vga std \
		$(QEMU_USB_INPUT) \
		-netdev user,id=net0 \
		-device virtio-net-pci,netdev=net0,disable-modern=on,disable-legacy=off \
		-display "$(QEMU_DISPLAY)" \
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
		--expect "FB 1920x1080" \
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
		--artifact-name "ui-golden-start-search" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_INTERACTIVE_SECONDS) \
		--hmp "sendkey ctrl-spc" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "color" \
		--post-hmp-delay 0.8 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/ui-golden-start-search.ppm" \
		--expect-framebuffer-start-menu \
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
		start-search="$(SMOKE_ARTIFACT_DIR)/ui-golden-start-search.ppm" \
		settings="$(SMOKE_ARTIFACT_DIR)/ui-golden-settings.ppm" \
		diagnostics="$(SMOKE_ARTIFACT_DIR)/ui-golden-diagnostics.ppm"

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
		--expect "sdkdemo: libcool sdk=1 abi=14" \
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
		--pre-type-delay 3.0 \
		--type-text "exec /bin/guidemo\n" \
		--post-hmp-delay 3.0 \
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
		--expect "[pkg] installed app.phase25.guidemo name=Packaged GUI Demo exec=/bin/pkgdemo payloads=1" \
		--expect "[pkg] launched app.phase25.guidemo exec=/bin/pkgdemo pid=" \
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

smoke-phase80-firstboot: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(SMOKE_ARTIFACT_DIR)/phase80-firstboot.img"
	cp "$(FSIMG)" "$(SMOKE_ARTIFACT_DIR)/phase80-firstboot.img"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-screen" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase80-firstboot.img" \
		--first-boot \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase80-firstboot-screen.ppm" \
		--expect-framebuffer-login \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-complete" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase80-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 60 \
		--no-auto-login \
		--interact-after "[boot] first boot ready" \
		--type-text "owner80\nownerpass80\nownerpass80\ncoolOS 80\n" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase80-firstboot-complete.ppm" \
		--expect-framebuffer-desktop \
		--expect "[install] first boot complete user=owner80" \
		--expect "[session] login owner80 uid=" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-persist" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase80-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--auto-login-text "ownerpass80\n" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase80-firstboot-persist.ppm" \
		--expect-framebuffer-desktop \
		--expect "[boot] login ready" \
		--expect "[session] login owner80 uid=" \
		--expect "[boot] desktop ready"

reset-firstboot-smoke-image: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FIRSTBOOT_RESET_IMG)" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--auto-login-text "$(FIRSTBOOT_RESET_LOGIN)" \
		--fw-cmd "recovery firstboot reset;;recovery firstboot status;;flush" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/reset-firstboot-smoke-image.ppm" \
		--expect "first-boot reset context=recovery" \
		--expect "first-run setup=required" \
		--expect "[boot] desktop ready"

smoke-phase81-firstboot-recovery: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img"
	cp "$(FSIMG)" "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-in-progress" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-resume" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot-resume.ppm" \
		--expect-framebuffer-login \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-complete" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 60 \
		--no-auto-login \
		--interact-after "[boot] first boot ready" \
		--type-text "owner81\nownerpass81\nownerpass81\ncoolOS 81\n" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot-complete.ppm" \
		--expect-framebuffer-desktop \
		--expect "[install] first boot complete user=owner81" \
		--expect "[session] login owner81 uid=" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-persist-flush" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--fw-cmd "login owner81 ownerpass81;;flush" \
		--expect "[session] login owner81 uid=" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-recovery-repair" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--fw-cmd "login owner81 ownerpass81;;recovery firstboot repair;;recovery firstboot status;;flush" \
		--expect "first-boot repair context=recovery" \
		--expect "repair=no-changes" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-nonadmin-denied" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--fw-cmd "login owner81 ownerpass81;;login guest guest;;install reset;;install repair;;flush" \
		--expect "session user guest uid=1001" \
		--expect "install: permission denied" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-reset" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--fw-cmd "login owner81 ownerpass81;;install reset;;install status;;flush" \
		--expect "first-boot reset context=admin" \
		--expect "first-run setup=required" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-post-reset" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot.img" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase81-firstboot-post-reset.ppm" \
		--expect-framebuffer-login \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"

smoke-phase82-installer: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-card" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase82-installer-card.ppm" \
		--expect-framebuffer-installer \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-install" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 300 \
		--fw-cmd "install disks;;install disk ide1-master;;install verify ide1-master;;flush" \
		--expect "installer mode=active" \
		--expect "ide0-slave present=yes" \
		--expect "role=root protected=yes installable=no" \
		--expect "ide1-master present=yes" \
		--expect "install complete target=ide1-master" \
		--expect "verify=ok" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-firstboot" \
		--bios "$(BIOS)" \
		--fsimg "$(INSTALL_TARGET_IMG)" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase82-target-firstboot.ppm" \
		--expect-framebuffer-login \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-owner" \
		--bios "$(BIOS)" \
		--fsimg "$(INSTALL_TARGET_IMG)" \
		--fs-writable \
		--first-boot \
		--usb \
		--seconds 60 \
		--no-auto-login \
		--interact-after "[boot] first boot ready" \
		--type-text "owner82\nownerpass82\nownerpass82\ncoolOS 82\n" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase82-target-owner.ppm" \
		--expect-framebuffer-desktop \
		--expect "[install] first boot complete user=owner82" \
		--expect "[session] login owner82 uid=" \
		--expect "[boot] desktop ready"

smoke-phase83-self-booting-installer: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-card" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase83-installer-card.ppm" \
		--expect-framebuffer-installer \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-install" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 300 \
		--fw-cmd "install disks;;install disk ide1-master;;install verify ide1-master;;flush" \
		--expect "installer mode=active" \
		--expect "ide0-master present=yes" \
		--expect "ide0-slave present=yes" \
		--expect "role=boot protected=yes installable=no" \
		--expect "role=root protected=yes installable=no" \
		--expect "ide1-master present=yes" \
		--expect "layout=self-boot" \
		--expect "mbr patched partition=3 type=0xc0" \
		--expect "install complete target=ide1-master" \
		--expect "verify=ok layout=self-boot" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-firstboot" \
		--boot-disk "$(INSTALL_TARGET_IMG)" \
		--boot-disk-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase83-target-firstboot.ppm" \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-owner" \
		--boot-disk "$(INSTALL_TARGET_IMG)" \
		--boot-disk-writable \
		--first-boot \
		--usb \
		--seconds 60 \
		--no-auto-login \
		--interact-after "[boot] first boot ready" \
		--type-text "owner83\nownerpass83\nownerpass83\ncoolOS 83\n" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase83-target-owner.ppm" \
		--expect-framebuffer-desktop \
		--expect "[install] first boot complete user=owner83" \
		--expect "[session] login owner83 uid=" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-reboot" \
		--boot-disk "$(INSTALL_TARGET_IMG)" \
		--first-boot \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase83-target-reboot.ppm" \
		--expect-framebuffer-login \
		--expect "[boot] login ready" \
		--expect "[boot] desktop ready"

smoke-phase84-installer-v2: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)" "$(INSTALL_SMALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_SMALL_TARGET_SIZE)" "$(INSTALL_SMALL_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-selection" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase84-installer-selection.ppm" \
		--expect-framebuffer-installer \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-plan" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 90 \
		--fw-cmd "install disks;;install plan ide1-master;;install plan ide0-master;;install plan ide0-slave;;flush" \
		--expect "installer mode=active" \
		--expect "plan target=ide1-master" \
		--expect "installable=yes reason=ready" \
		--expect "plan target=ide0-master" \
		--expect "installable=no reason=refusing to overwrite boot disk" \
		--expect "plan target=ide0-slave" \
		--expect "installable=no reason=refusing to overwrite mounted root disk" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-small-target" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_SMALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 90 \
		--fw-cmd "install plan ide1-master;;install disk ide1-master;;flush" \
		--expect "plan target=ide1-master" \
		--expect "installable=no reason=target too small" \
		--expect "install: target too small" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-gui-install" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 300 \
		--no-auto-login \
		--interact-after "[boot] installer ready" \
		--type-text "\nide1-master\n" \
		--post-hmp-delay 1.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase84-installer-progress.ppm" \
		--expect-framebuffer-installer \
		--expect "[install] gui install started target=ide1-master" \
		--expect "[install] gui install complete target=ide1-master" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-firstboot" \
		--boot-disk "$(INSTALL_TARGET_IMG)" \
		--boot-disk-writable \
		--first-boot \
		--usb \
		--seconds 45 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase84-target-firstboot.ppm" \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"

smoke-phase85-uefi-gpt: build-uefi
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-live" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--bios "$(UEFI)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 60 \
		--expect "FB 1920x1080" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-installer" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--bios "$(UEFI)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 60 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase85-uefi-installer.ppm" \
		--expect-framebuffer-installer \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-install" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--bios "$(UEFI)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 300 \
		--fw-cmd "install disks;;install plan ide1-master;;install plan ide0-master;;install plan ide0-slave;;install disk ide1-master;;install verify ide1-master;;flush" \
		--expect "installer mode=active" \
		--expect "role=boot protected=yes installable=no" \
		--expect "role=root protected=yes installable=no" \
		--expect "layout=uefi-gpt" \
		--expect "gpt patched" \
		--expect "install complete target=ide1-master" \
		--expect "verify=ok layout=uefi-gpt" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-firstboot" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(INSTALL_TARGET_IMG)" \
		--boot-disk-writable \
		--first-boot \
		--usb \
		--seconds 60 \
		--no-auto-login \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase85-target-firstboot.ppm" \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"

smoke-phase86-ahci-storage: build-uefi build-usb-image
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-live" \
		--uefi \
		--ahci \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--bios "$(UEFI)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds 75 \
		--expect "FB 1920x1080" \
		--expect "AHCI: sata0 present" \
		--expect "AHCI: sata1 present" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-install" \
		--uefi \
		--ahci \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--bios "$(UEFI)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-writable \
		--installer \
		--usb \
		--seconds 300 \
		--fw-cmd "install disks;;install plan sata2;;install plan sata0;;install plan sata1;;install disk sata2;;install verify sata2;;flush" \
		--expect "installer mode=active" \
		--expect "sata0 present=yes" \
		--expect "sata1 present=yes" \
		--expect "sata2 present=yes" \
		--expect "plan target=sata2" \
		--expect "installable=yes reason=ready" \
		--expect "plan target=sata0" \
		--expect "installable=no reason=refusing to overwrite boot disk" \
		--expect "plan target=sata1" \
		--expect "installable=no reason=refusing to overwrite mounted root disk" \
		--expect "layout=uefi-gpt" \
		--expect "install complete target=sata2" \
		--expect "verify=ok layout=uefi-gpt" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-firstboot" \
		--uefi \
		--ahci \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(INSTALL_TARGET_IMG)" \
		--boot-disk-writable \
		--first-boot \
		--usb \
		--seconds 75 \
		--no-auto-login \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "AHCI: sata0 present" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-usb-image" \
		--uefi \
		--ahci \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_IMAGE)" \
		--first-boot \
		--usb \
		--seconds 75 \
		--no-auto-login \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "AHCI: sata0 present" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"

smoke-phase87-usb-storage-root: build-usb-image
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_IMAGE)" \
		--usb-storage \
		--first-boot \
		--usb \
		--seconds 90 \
		--no-auto-login \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"

smoke-phase88-nvme-storage: build-uefi build-usb-image
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(INSTALL_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(INSTALL_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-root" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_IMAGE)" \
		--nvme \
		--first-boot \
		--usb \
		--seconds 90 \
		--no-auto-login \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "nvme0n1 present sectors=" \
		--expect "[storage] root device=nvme0n1 layout=gpt-coolfs" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-install" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--bios "$(UEFI)" \
		--fsimg "$(FSIMG)" \
		--target-disk "$(INSTALL_TARGET_IMG)" \
		--target-nvme \
		--target-writable \
		--installer \
		--usb \
		--seconds 300 \
		--fw-cmd "install disks;;install plan nvme0n1;;install disk nvme0n1;;install verify nvme0n1;;flush" \
		--expect "installer mode=active" \
		--expect "nvme0n1 present=yes" \
		--expect "plan target=nvme0n1" \
		--expect "installable=yes reason=ready" \
		--expect "layout=uefi-gpt" \
		--expect "install complete target=nvme0n1" \
		--expect "verify=ok layout=uefi-gpt" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-firstboot" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(INSTALL_TARGET_IMG)" \
		--boot-disk-writable \
		--nvme \
		--first-boot \
		--usb \
		--seconds 90 \
		--no-auto-login \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "nvme0n1 present sectors=" \
		--expect "[storage] root device=nvme0n1 layout=gpt-coolfs" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"

smoke-phase89-baremetal-readiness: build-usb-image build-usb-safe-image
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-usb-normal" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_IMAGE)" \
		--usb-storage \
		--first-boot \
		--usb \
		--seconds 90 \
		--no-auto-login \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-usb-safe" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_SAFE_IMAGE)" \
		--usb-storage \
		--safe-mode \
		--first-boot \
		--usb \
		--seconds 90 \
		--no-auto-login \
		--expect-framebuffer-login \
		--expect "[boot] safe mode" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-hardware" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_IMAGE)" \
		--usb-storage \
		--usb \
		--seconds 120 \
		--fw-cmd "hardware;;devices;;sysreport;;flush" \
		--expect "HARDWARE" \
		--expect "hardware mode=normal" \
		--expect "storage root=usb0 layout=gpt-coolfs" \
		--expect "USB: runtime" \
		--expect "SYSREPORT" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"

smoke-phase90-physical-installer: build-usb-image
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(PHYSICAL_AHCI_TARGET_IMG)" "$(PHYSICAL_NVME_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(PHYSICAL_AHCI_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(PHYSICAL_NVME_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-install" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_IMAGE)" \
		--usb-storage \
		--target-disk "$(PHYSICAL_NVME_TARGET_IMG)" \
		--target-nvme \
		--target-writable \
		--ahci-target-disk "$(PHYSICAL_AHCI_TARGET_IMG)" \
		--ahci-target-writable \
		--ahci-target-port 0 \
		--installer \
		--usb \
		--seconds 360 \
		--fw-cmd "install disks;;install plan usb0;;install plan sata0;;install plan nvme0n1;;install physical nvme0n1;;install verify nvme0n1;;hardware;;flush" \
		--expect "installer mode=active" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "usb0 present=yes" \
		--expect "bus=usb role=usb-installer protected=yes installable=no" \
		--expect "reason=refusing to overwrite mounted root disk" \
		--expect "sata0 present=yes" \
		--expect "bus=sata role=physical-target" \
		--expect "nvme0n1 present=yes" \
		--expect "bus=nvme role=physical-target" \
		--expect "source_mode=usb-live" \
		--expect "installable=yes reason=ready" \
		--expect "layout=uefi-gpt" \
		--expect "install physical target=nvme0n1" \
		--expect "install complete target=nvme0n1" \
		--expect "verify=ok layout=uefi-gpt" \
		--expect "installer candidate=nvme0n1 bus=nvme role=physical-target" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-target-firstboot" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(PHYSICAL_NVME_TARGET_IMG)" \
		--boot-disk-writable \
		--nvme \
		--first-boot \
		--usb \
		--seconds 90 \
		--no-auto-login \
		--expect-framebuffer-login \
		--expect "FB 1920x1080" \
		--expect "nvme0n1 present sectors=" \
		--expect "[storage] root device=nvme0n1 layout=gpt-coolfs" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"

smoke-phase91-hardware-readiness: build-usb-image
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(PHASE91_UASP_IMG)" "$(PHYSICAL_AHCI_TARGET_IMG)" "$(PHYSICAL_NVME_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(PHASE91_UASP_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(PHYSICAL_AHCI_TARGET_IMG)"
	truncate -s "$(INSTALL_TARGET_SIZE)" "$(PHYSICAL_NVME_TARGET_IMG)"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-usb-topology" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_IMAGE)" \
		--usb-storage \
		--usb-hub \
		--usb-uas-disk "$(PHASE91_UASP_IMG)" \
		--usb \
		--seconds 140 \
		--fw-cmd "hardware;;devices;;sysreport;;flush" \
		--expect "MSC usb0" \
		--expect "UASP unsupported" \
		--expect "hub iface=" \
		--expect "storage root=usb0 layout=gpt-coolfs" \
		--expect "storage root_scan device=usb0" \
		--expect "state=gpt-coolfs" \
		--expect "SYSREPORT" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-installer-preflight" \
		--uefi \
		--uefi-code "$(QEMU_EFI_CODE)" \
		--boot-disk "$(USB_IMAGE)" \
		--usb-storage \
		--target-disk "$(PHYSICAL_NVME_TARGET_IMG)" \
		--target-nvme \
		--target-writable \
		--ahci-target-disk "$(PHYSICAL_AHCI_TARGET_IMG)" \
		--ahci-target-writable \
		--ahci-target-port 0 \
		--installer \
		--usb \
		--seconds 180 \
		--fw-cmd "install disks;;install plan usb0;;install plan sata0;;install plan nvme0n1;;hardware;;flush" \
		--expect "installer mode=active" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "sata0 present=yes" \
		--expect "nvme0n1 present=yes" \
		--expect "installer preflight verdict=ok reason=ready" \
		--expect "internal_targets=2" \
		--expect "installable_targets=2" \
		--expect "AHCI:" \
		--expect "NVMe:" \
		--expect "storage root_scan device=usb0" \
		--expect "flush: ok" \
		--expect "[boot] installer ready" \
		--expect "[boot] desktop ready"

smoke-phase92-secure-boot: build-usb-secure-image
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-firstboot" \
		--uefi-secure \
		--uefi-code "$(QEMU_EFI_SECURE_CODE)" \
		--uefi-vars "$(QEMU_EFI_SECURE_VARS)" \
		--boot-disk "$(USB_SECURE_IMAGE)" \
		--usb-storage \
		--first-boot \
		--usb \
		--seconds 120 \
		--no-auto-login \
		--expect-framebuffer-login \
		--secure-boot-status "$(QEMU_SECURE_STATUS)" \
		--expect "FB 1920x1080" \
		--expect "[secureboot] $(QEMU_SECURE_STATUS)" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-diagnostics" \
		--uefi-secure \
		--uefi-code "$(QEMU_EFI_SECURE_CODE)" \
		--uefi-vars "$(QEMU_EFI_SECURE_VARS)" \
		--boot-disk "$(USB_SECURE_IMAGE)" \
		--usb-storage \
		--usb \
		--seconds 120 \
		--secure-boot-status "$(QEMU_SECURE_STATUS)" \
		--fw-cmd "hardware;;sysreport;;flush" \
		--expect "FB 1920x1080" \
		--expect "[secureboot] $(QEMU_SECURE_STATUS)" \
		--expect "secure_boot $(QEMU_SECURE_STATUS)" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "storage root=usb0 layout=gpt-coolfs" \
		--expect "SYSREPORT" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"

smoke-phase93-secure-boot: build-usb-image build-usb-secure-image verify-secure-boot-artifacts tamper-secure-boot-artifacts
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-firstboot" \
		--uefi-secure \
		--uefi-code "$(QEMU_EFI_SECURE_CODE)" \
		--uefi-vars "$(QEMU_EFI_SECURE_VARS)" \
		--boot-disk "$(USB_SECURE_IMAGE)" \
		--usb-storage \
		--first-boot \
		--usb \
		--seconds 120 \
		--no-auto-login \
		--expect-framebuffer-login \
		--secure-boot-status "$(QEMU_SECURE_STATUS)" \
		--expect "FB 1920x1080" \
		--expect "[secureboot] $(QEMU_SECURE_STATUS)" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "[boot] first boot ready" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-diagnostics" \
		--uefi-secure \
		--uefi-code "$(QEMU_EFI_SECURE_CODE)" \
		--uefi-vars "$(QEMU_EFI_SECURE_VARS)" \
		--boot-disk "$(USB_SECURE_IMAGE)" \
		--usb-storage \
		--usb \
		--seconds 120 \
		--secure-boot-status "$(QEMU_SECURE_STATUS)" \
		--fw-cmd "hardware;;sysreport;;flush" \
		--expect "FB 1920x1080" \
		--expect "[secureboot] $(QEMU_SECURE_STATUS)" \
		--expect "secure_boot $(QEMU_SECURE_STATUS)" \
		--expect "MSC usb0" \
		--expect "[storage] root device=usb0 layout=gpt-coolfs" \
		--expect "storage root=usb0 layout=gpt-coolfs" \
		--expect "SYSREPORT" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	@if python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-unsigned-loader-negative" \
		--uefi-secure \
		--uefi-code "$(QEMU_EFI_SECURE_CODE)" \
		--uefi-vars "$(QEMU_EFI_SECURE_VARS)" \
		--boot-disk "$(USB_IMAGE)" \
		--usb-storage \
		--usb \
		--seconds 24 \
		--expect "[boot] desktop ready"; then \
		echo "expected Secure Boot to reject unsigned BOOTX64.EFI"; exit 1; \
	else \
		echo "secure boot negative ok: unsigned loader rejected"; \
	fi
	@if python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-tampered-loader-negative" \
		--uefi-secure \
		--uefi-code "$(QEMU_EFI_SECURE_CODE)" \
		--uefi-vars "$(QEMU_EFI_SECURE_VARS)" \
		--boot-disk "$(USB_SECURE_LOADER_TAMPER_IMAGE)" \
		--usb-storage \
		--usb \
		--seconds 24 \
		--expect "[boot] desktop ready"; then \
		echo "expected Secure Boot to reject tampered signed BOOTX64.EFI"; exit 1; \
	else \
		echo "secure boot negative ok: tampered signed loader rejected"; \
	fi
	@if python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-tampered-kernel-negative" \
		--uefi-secure \
		--uefi-code "$(QEMU_EFI_SECURE_CODE)" \
		--uefi-vars "$(QEMU_EFI_SECURE_VARS)" \
		--boot-disk "$(USB_SECURE_KERNEL_TAMPER_IMAGE)" \
		--usb-storage \
		--usb \
		--seconds 24 \
		--expect "[boot] desktop ready"; then \
		echo "expected signed loader to reject kernel digest mismatch"; exit 1; \
	else \
		echo "secure boot negative ok: kernel digest mismatch rejected"; \
	fi

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
		--interact-after "sh: ready abi=14" \
		--type-text "pwd\nls /bin\ncat /bin/hello.txt\necho userspace shell ok\nexit\n" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "foreground /bin/sh" \
		--expect "sh: ready abi=14" \
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
		--interact-after "sh: ready abi=14" \
		--type-text "run /bin/pwd\nrun /bin/echo external coreutils ok\nrun /bin/ls /bin\nrun /bin/cat /bin/hello.txt\nrun /bin/mkdir /TMP/PH37\nrun /bin/writefile /TMP/PH37/NOTE coreutils file ok\nrun /bin/cat /TMP/PH37/NOTE\nrun /bin/touch /TMP/PH37/EMPTY\nrun /bin/rm /TMP/PH37/EMPTY\nrun /bin/rm /TMP/PH37\nexit\n" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "foreground /bin/sh" \
		--expect "sh: ready abi=14" \
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
		--interact-after "sh: ready abi=14" \
		--type-key-delay $(SMOKE_TYPE_KEY_DELAY) \
		--type-text "cd /TMP\npwd\necho phase40 redirect ok > p40.txt\ncat p40.txt\ncat p40.txt | grep redirect\ncp p40.txt p40-copy.txt\nmv p40-copy.txt p40-moved.txt\nstat p40-moved.txt\nuname\ndate\nsync\nexit\n" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "sh: ready abi=14" \
		--expect "/tmp" \
		--expect "phase40 redirect ok" \
		--expect "kind=file size=" \
		--expect "coolOS coolOS-userspace-abi/14 x86_64" \
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
		--interact-after "sh: ready abi=14" \
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
		--interact-after "sh: ready abi=14" \
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
		--expect "coolOS devkit ABI=14" \
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
		--fw-cmd "devkit;;cat /SDK/README.TXT;;cat /SDK/BROWSER_ENGINE_PORT.TXT;;exec /bin/devkit" \
		--expect "coolOS devkit ABI=14" \
		--expect "coolOS SDK" \
		--expect "ABI version: 14" \
		--expect "Target engine: WPE WebKit." \
		--expect "foreground /bin/devkit" \
		--expect "template: /SDK/APP_TEMPLATE.RS" \
		--expect "browser engine: /SDK/BROWSER_ENGINE_PORT.TXT" \
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

smoke-pointer-tablet: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--usb-tablet \
		--seconds 35 \
		--retries $(SMOKE_RETRIES) \
		--fw-cmd "smoothness" \
		--expect "hid tablet" \
		--expect "COMPOSITOR" \
		--expect "active_hz=144" \
		--expect "cursor_mode=overlay" \
		--expect "pointer_kind=tablet" \
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
		--expect "polldemo: abi=14" \
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
		--expect "tuidemo: abi=14" \
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

smoke-phase57-browser-layout: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE57.LAYOUT.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase57-browser-layout.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase57.layout.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase58-browser-subresources: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE58.SUBRESOURCES.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase58-browser-subresources.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase58.subresources.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase59-browser-js: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE59.JS.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase59-browser-js.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase59.js.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase60-browser-webapi: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE60.WEBAPP.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase60-browser-webapi.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase60.webapp.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase61-browser-compat: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--hmp "sendkey ctrl-n" \
		--pre-type-delay $(SMOKE_PRE_TYPE_DELAY) \
		--type-text "browser file:///TMP/PHASE61.GOOGLE.HTML\n" \
		--post-hmp-delay 2.0 \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase61-browser-compat.ppm" \
		--expect-framebuffer-window \
		--expect "[browser] open file:///tmp/phase61.google.html" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase62-resource-limits: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "diagnostics" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase62-resource-limits.ppm" \
		--expect-framebuffer-window \
		--expect "== resource limits ==" \
		--expect "tasks active=" \
		--expect "address-space owned_pages=" \
		--expect "vfs fd_tables=" \
		--expect "net sockets=" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase63-memory-pressure: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "memory;;diagnostics" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase63-memory-pressure.ppm" \
		--expect-framebuffer-window \
		--expect "MEMORY PRESSURE" \
		--expect "heap pressure=" \
		--expect "== task memory ==" \
		--expect "reclaim checks=" \
		--expect "oom kills=" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase64-services: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "services recovery;;services stop search-index;;services status search-index;;cat /CONFIG/SERVICES.CFG;;services start search-index;;services fail package-db;;services run;;services history;;sysreport write;;cat /LOGS/SYSREPORT.TXT" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase64-services.ppm" \
		--expect-framebuffer-window \
		--expect "SERVICE RECOVERY" \
		--expect "config=/CONFIG/SERVICES.CFG history=/LOGS/SERVICES.TXT" \
		--expect "search-index state=stopped restart=manual" \
		--expect "search-index state=running restart=manual" \
		--expect "service supervisor tick" \
		--expect "SERVICE HISTORY" \
		--expect "reason=supervisor-restart" \
		--expect "== service recovery ==" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase65-update-rollback: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/PHASE65.TXT before;;update apply;;hash /CONFIG/PHASE65.TXT;;update stage /CONFIG/PHASE65.TXT after;;update status;;update apply;;hash /CONFIG/PHASE65.TXT;;recovery rollback;;hash /CONFIG/PHASE65.TXT;;update history;;sysreport" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase65-update-rollback.ppm" \
		--expect-framebuffer-window \
		--expect "update: staged" \
		--expect "UPDATE STATUS" \
		--expect "staged=yes id=manual version=2 files=1 services=search-index,package-db" \
		--expect "update: applied" \
		--expect "hash /CONFIG/PHASE65.TXT len=6 sum=627" \
		--expect "hash /CONFIG/PHASE65.TXT len=5 sum=530" \
		--expect "UPDATE HISTORY" \
		--expect "action=apply-ok" \
		--expect "RECOVERY ROLLBACK" \
		--expect "update rollback ok" \
		--expect "action=rollback-ok" \
		--expect "== updates ==" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase66-boot-health: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(SMOKE_ARTIFACT_DIR)/phase66-boot-health.img"
	cp "$(FSIMG)" "$(SMOKE_ARTIFACT_DIR)/phase66-boot-health.img"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-arm" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase66-boot-health.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/PHASE66.TXT before;;update apply;;boot mark-good;;update stage /CONFIG/PHASE66.TXT after;;update apply;;boot fail-validation manual smoke;;boot status;;hash /CONFIG/PHASE66.TXT;;flush" \
		--expect "update: applied" \
		--expect "boot: marked good" \
		--expect "boot: validation failure recorded" \
		--expect "status=validating pending_update=manual attempts=1" \
		--expect "hash /CONFIG/PHASE66.TXT len=5 sum=530" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-recover" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase66-boot-health.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "boot status;;hash /CONFIG/PHASE66.TXT;;boot history;;update history;;recovery;;sysreport" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase66-boot-health.ppm" \
		--expect-framebuffer-window \
		--expect "[boot-health] auto rollback ok id=manual" \
		--expect "BOOT HEALTH" \
		--expect "status=healthy pending_update=none attempts=0" \
		--expect "last_auto_rollback=manual" \
		--expect "hash /CONFIG/PHASE66.TXT len=6 sum=627" \
		--expect "BOOT HISTORY" \
		--expect "action=auto-rollback-ok" \
		--expect "UPDATE HISTORY" \
		--expect "action=rollback-ok" \
		--expect "boot_health status=healthy pending_update=none attempts=0" \
		--expect "== boot health ==" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase67-update-trust: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(SMOKE_ARTIFACT_DIR)/phase67-update-trust.img"
	cp "$(FSIMG)" "$(SMOKE_ARTIFACT_DIR)/phase67-update-trust.img"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-valid" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase67-update-trust.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update keys;;update stage /CONFIG/P67 ok;;update verify;;update apply;;hash /CONFIG/P67;;flush" \
		--expect "UPDATE TRUST KEYS" \
		--expect "keys=/CONFIG/UPDATE-KEYS.TXT built_in=4 signature_required=yes rotation=yes anti_rollback=yes" \
		--expect "key=phase68-root-a algorithm=ed25519 status=trusted scope=staged-updates" \
		--expect "UPDATE VERIFY" \
		--expect "trust=ok key=phase68-root-a algorithm=ed25519 version=1 files=1" \
		--expect "update: applied" \
		--expect "hash /CONFIG/P67 len=2 sum=218" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-tamper" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase67-update-trust.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/P67 bad;;update corrupt-payload evil;;update verify;;update apply;;hash /CONFIG/P67;;flush" \
		--expect "update: payload corrupted" \
		--expect "trust=failed error=payload hash mismatch" \
		--expect "update: payload hash mismatch" \
		--expect "hash /CONFIG/P67 len=2 sum=218" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-unsigned" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase67-update-trust.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/P67 new;;update unsign;;update verify;;update apply;;hash /CONFIG/P67;;flush" \
		--expect "update: unsigned" \
		--expect "trust=failed error=staged update is unsigned" \
		--expect "update: staged update is unsigned" \
		--expect "hash /CONFIG/P67 len=2 sum=218" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-rollback" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase67-update-trust.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/P67 new;;update apply;;hash /CONFIG/P67;;update rollback;;hash /CONFIG/P67;;update status;;recovery;;sysreport" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase67-update-trust.ppm" \
		--expect-framebuffer-window \
		--expect "update: applied" \
		--expect "hash /CONFIG/P67 len=3 sum=330" \
		--expect "update: rollback ok" \
		--expect "hash /CONFIG/P67 len=2 sum=218" \
		--expect "trust=ok key=phase68-root-a algorithm=ed25519 version=2 files=1" \
		--expect "update_trust=ok key=phase68-root-a algorithm=ed25519 version=2 files=1" \
		--expect "== updates ==" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase68-update-keys: build
	mkdir -p "$(SMOKE_ARTIFACT_DIR)"
	rm -f "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.img"
	cp "$(FSIMG)" "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.img"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-valid-root-a" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update keys;;update stage /CONFIG/P68 one;;update verify;;update apply;;hash /CONFIG/P68;;flush" \
		--expect "UPDATE TRUST KEYS" \
		--expect "key=phase68-root-a algorithm=ed25519 status=trusted scope=staged-updates" \
		--expect "key=phase68-root-b algorithm=ed25519 status=trusted scope=staged-updates" \
		--expect "key=phase68-revoked algorithm=ed25519 status=revoked scope=staged-updates" \
		--expect "key=phase68-expired algorithm=ed25519 status=trusted scope=staged-updates" \
		--expect "trust=ok key=phase68-root-a algorithm=ed25519 version=1 files=1" \
		--expect "update: applied" \
		--expect "hash /CONFIG/P68 len=3 sum=322" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-downgrade" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage-version /CONFIG/P68 1 old;;update verify;;update apply;;hash /CONFIG/P68;;flush" \
		--expect "trust=failed error=update version rollback" \
		--expect "update: update version rollback" \
		--expect "hash /CONFIG/P68 len=3 sum=322" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-rotated-root-b" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/P68 two;;update sign-as phase68-root-b;;update verify;;update apply;;hash /CONFIG/P68;;cat /UPDATES/APPLIED.MF;;flush" \
		--expect "update: signed as phase68-root-b" \
		--expect "trust=ok key=phase68-root-b algorithm=ed25519 version=2 files=1" \
		--expect "update: applied" \
		--expect "hash /CONFIG/P68 len=3 sum=346" \
		--expect "verified_by=phase68-root-b" \
		--expect "algorithm=ed25519" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-revoked" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/P68 bad;;update sign-as phase68-revoked;;update verify;;update apply;;hash /CONFIG/P68;;flush" \
		--expect "update: signed as phase68-revoked" \
		--expect "trust=failed error=update key revoked" \
		--expect "update: update key revoked" \
		--expect "hash /CONFIG/P68 len=3 sum=346" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-expired" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/P68 old;;update sign-as phase68-expired;;update verify;;update apply;;hash /CONFIG/P68;;flush" \
		--expect "update: signed as phase68-expired" \
		--expect "trust=failed error=update key expired" \
		--expect "update: update key expired" \
		--expect "hash /CONFIG/P68 len=3 sum=346" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-unknown" \
		--bios "$(BIOS)" \
		--fsimg "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.img" \
		--fs-writable \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "update stage /CONFIG/P68 unk;;update sign-as phase68-unknown;;update verify;;update apply;;hash /CONFIG/P68;;update history;;recovery;;sysreport" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase68-update-keys.ppm" \
		--expect-framebuffer-window \
		--expect "update: signed as phase68-unknown" \
		--expect "trust=failed error=update key not trusted" \
		--expect "update: update key not trusted" \
		--expect "hash /CONFIG/P68 len=3 sum=346" \
		--expect "UPDATE HISTORY" \
		--expect "action=verify-failed" \
		--expect "update_trust=failed error=update key not trusted" \
		--expect "== updates ==" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase69-package-trust: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-valid" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg keys;;pkg verify /Packages/guidemo.pkg;;pkg info /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;pkg verify pkgdemo;;pkg info pkgdemo;;cat /APPS/pkgdemo/OWNER.TXT;;flush" \
		--expect "PACKAGE TRUST KEYS" \
		--expect "key=phase69-pkg-a algorithm=ed25519 status=trusted scope=packages" \
		--expect "key=phase69-pkg-b algorithm=ed25519 status=trusted scope=packages" \
		--expect "key=phase69-pkg-revoked algorithm=ed25519 status=revoked scope=packages" \
		--expect "key=phase69-pkg-expired algorithm=ed25519 status=trusted scope=packages" \
		--expect "package_trust=ok key=phase69-pkg-a algorithm=ed25519 id=app.phase25.guidemo version=1.0 command=pkgdemo" \
		--expect "[pkg] installed app.phase25.guidemo name=Packaged GUI Demo exec=/bin/pkgdemo payloads=1" \
		--expect "installed_trust=ok id=app.phase25.guidemo command=pkgdemo version=1.0 key=phase69-pkg-a algorithm=ed25519" \
		--expect "payloads=ok count=1" \
		--expect "payload=/bin/pkgdemo|source=/Packages/guidemo.elf" \
		--expect "verified_by=phase69-pkg-a" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-rotated" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg sign-as /Packages/guidemo.pkg phase69-pkg-b;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;cat /APPS/pkgdemo/OWNER.TXT;;flush" \
		--expect "pkg: ok" \
		--expect "package_trust=ok key=phase69-pkg-b algorithm=ed25519 id=app.phase25.guidemo version=1.0 command=pkgdemo" \
		--expect "[pkg] installed app.phase25.guidemo name=Packaged GUI Demo exec=/bin/pkgdemo payloads=1" \
		--expect "payload=/bin/pkgdemo|/Packages/guidemo.elf" \
		--expect "verified_by=phase69-pkg-b" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-unsigned-tampered" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg unsign /Packages/guidemo.pkg;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;pkg sign /Packages/guidemo.pkg;;pkg tamper /Packages/guidemo.pkg;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;flush" \
		--expect "package_trust=failed error=package is unsigned" \
		--expect "pkg: package is unsigned" \
		--expect "package_trust=failed error=package manifest hash mismatch" \
		--expect "pkg: package manifest hash mismatch" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-bad-keys" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg sign-as /Packages/guidemo.pkg phase69-pkg-revoked;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;pkg sign-as /Packages/guidemo.pkg phase69-pkg-expired;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;flush" \
		--expect "package_trust=failed error=package key revoked" \
		--expect "pkg: package key revoked" \
		--expect "package_trust=failed error=package key expired" \
		--expect "pkg: package key expired" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-unknown-key" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg sign-as /Packages/guidemo.pkg phase69-pkg-unknown;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;flush" \
		--expect "package_trust=failed error=package key not trusted" \
		--expect "pkg: package key not trusted" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-deps" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg deps /Packages/guidemo.pkg app.missing;;pkg sign /Packages/guidemo.pkg;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;flush" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase69-package-trust.ppm" \
		--expect-framebuffer-window \
		--expect "dependencies=missing app.missing" \
		--expect "pkg: package dependency missing" \
		--expect "flush: ok" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-repair-report" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg install /Packages/guidemo.pkg;;pkg break pkgdemo;;pkg verify pkgdemo;;pkg repair pkgdemo;;pkg verify pkgdemo;;pkg remove pkgdemo;;pkg verify pkgdemo;;pkg history;;recovery;;sysreport" \
		--expect "[pkg] installed app.phase25.guidemo name=Packaged GUI Demo exec=/bin/pkgdemo payloads=1" \
		--expect "installed_trust=failed error=installed manifest invalid" \
		--expect "installed_trust=ok id=app.phase25.guidemo command=pkgdemo version=1.0 key=phase69-pkg-a algorithm=ed25519" \
		--expect "[pkg] removed app.phase25.guidemo" \
		--expect "installed_trust=failed error=package owner missing" \
		--expect "PACKAGE HISTORY" \
		--expect "package_trust=ok signed=0" \
		--expect "== packages ==" \
		--expect "[selftest] kernel unit checks ok=27 fail=0" \
		--expect "[boot] desktop ready"

smoke-phase70-package-payloads: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-valid-remove" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg info /Packages/guidemo.pkg;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg;;pkg verify pkgdemo;;perm /bin/pkgdemo;;pkg run pkgdemo;;pkg remove pkgdemo;;hash /bin/pkgdemo" \
		--expect "payloads=1" \
		--expect "payloads=ok count=1" \
		--expect "payload=/bin/pkgdemo|source=/Packages/guidemo.elf" \
		--expect "[pkg] installed app.phase25.guidemo name=Packaged GUI Demo exec=/bin/pkgdemo payloads=1" \
		--expect "installed_trust=ok id=app.phase25.guidemo command=pkgdemo version=1.0 key=phase69-pkg-a algorithm=ed25519" \
		--expect "/bin/pkgdemo file uid=0 gid=0 mode=755" \
		--expect "[pkg] launched app.phase25.guidemo exec=/bin/pkgdemo pid=" \
		--expect "guidemo: window opened" \
		--expect "[pkg] removed app.phase25.guidemo" \
		--expect "hash: file not found" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-repair-transaction" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg install /Packages/guidemo.pkg;;pkg break-payload pkgdemo;;pkg verify pkgdemo;;pkg repair pkgdemo;;pkg verify pkgdemo;;pkg transaction;;recovery;;sysreport" \
		--expect "installed_trust=failed error=installed payload hash mismatch" \
		--expect "installed_trust=ok id=app.phase25.guidemo command=pkgdemo version=1.0 key=phase69-pkg-a algorithm=ed25519" \
		--expect "payloads=ok count=1" \
		--expect "transaction=clean action=repair id=app.phase25.guidemo" \
		--expect "package_trust=ok signed=1" \
		--expect "== packages ==" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-source-tamper" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg tamper-payload /Packages/guidemo.pkg;;pkg verify /Packages/guidemo.pkg;;pkg install /Packages/guidemo.pkg" \
		--expect "package_trust=failed error=package payload hash mismatch" \
		--expect "pkg: package payload hash mismatch" \
		--expect "[boot] desktop ready"
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@-rollback" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "pkg install-fail /Packages/guidemo.pkg;;pkg transaction;;hash /bin/pkgdemo" \
		--expect "pkg: package transaction rollback" \
		--expect "transaction=rolled-back action=install id=app.phase25.guidemo" \
		--expect "hash: file not found" \
		--expect "[boot] desktop ready"

smoke-phase71-browser-engine-port: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "engine;;engine abi;;engine requirements;;browser browser://engine;;diagnostics;;recovery;;sysreport write;;cat /LOGS/SYSREPORT.TXT;;cat /SDK/BROWSER_ENGINE_PORT.TXT" \
		--screendump "$(SMOKE_ARTIFACT_DIR)/phase71-browser-engine-port.ppm" \
		--expect-framebuffer-window \
		--expect "BROWSER ENGINE PORT" \
		--expect "engine-port abi=1 target=wpe-webkit fallback=coolos-native active=coolos-native" \
		--expect "surface=rgba-shmem" \
		--expect "req.threads-futex=ready" \
		--expect "backend_probe=/SYSTEM/BROWSER-ENGINE/WPE.READY" \
		--expect "browser: opening browser://engine" \
		--expect "== browser engine ==" \
		--expect "browser_engine=port-prep target=wpe-webkit active=coolos-native abi=1" \
		--expect "Target engine: WPE WebKit." \
		--expect "[boot] desktop ready"

smoke-phase72-threads-futex: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "exec /bin/threaddemo;;diagnostics;;engine requirements;;sysreport write;;cat /LOGS/SYSREPORT.TXT" \
		--expect "threaddemo: abi=14" \
		--expect "threaddemo: spawned" \
		--expect "threaddemo: futex woke done=2 sum=72" \
		--expect "threaddemo: join 21 51" \
		--expect "threaddemo: phase72 ok" \
		--expect "futex waiters=0" \
		--expect "req.threads-futex=ready" \
		--expect "wrote /LOGS/SYSREPORT.TXT" \
		--expect "[boot] desktop ready"

smoke-phase73-tls-pthread: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "exec /bin/tlsdemo;;abi;;diagnostics;;engine requirements;;sysreport write;;cat /LOGS/SYSREPORT.TXT;;cat /SDK/BROWSER_ENGINE_PORT.TXT" \
		--expect "tlsdemo: abi=14" \
		--expect "tlsdemo: main tls base=" \
		--expect "tlsdemo: spawned" \
		--expect "tlsdemo: done=2 sum=102 once=1" \
		--expect "tlsdemo: join 1 2" \
		--expect "tlsdemo: phase73 ok" \
		--expect "req.threads-futex=ready" \
		--expect "thread_tls_set" \
		--expect "wrote /LOGS/SYSREPORT.TXT" \
		--expect "[boot] desktop ready"

smoke-phase74-pthread-libc: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "exec /bin/pthreaddemo;;abi;;diagnostics;;engine requirements;;engine log;;sysreport write;;cat /LOGS/SYSREPORT.TXT;;cat /SDK/README.TXT;;cat /SDK/BROWSER_ENGINE_PORT.TXT" \
		--expect "pthreaddemo: abi=14" \
		--expect "pthreaddemo: main tid=" \
		--expect "pthreaddemo: spawned" \
		--expect "pthreaddemo: done=2 sum=72 once=1 errno=5" \
		--expect "pthreaddemo: join 21 51" \
		--expect "pthreaddemo: nanosleep ok" \
		--expect "pthreaddemo: phase74 ok" \
		--expect "req.threads-futex=ready" \
		--expect "posix_libc=partial" \
		--expect "pthread_create" \
		--expect "wrote /LOGS/SYSREPORT.TXT" \
		--expect "[boot] desktop ready"

smoke-phase75-dynlink: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "exec /bin/lddemo;;abi;;engine requirements;;engine log;;sysreport write;;cat /LOGS/SYSREPORT.TXT;;cat /SDK/README.TXT;;cat /SDK/BROWSER_ENGINE_PORT.TXT" \
		--expect "lddemo: abi=14" \
		--expect "lddemo: loaded /lib/libphase75.so base=" \
		--expect "lddemo: symbol phase75_add=" \
		--expect "increment=9 result=42" \
		--expect "lddemo: phase75 ok" \
		--expect "coolOS-userspace-abi version 14" \
		--expect "req.dynamic-linker=partial" \
		--expect "req.jit-execmem=partial" \
		--expect "dynamic_linker=partial" \
		--expect "dynlink::load" \
		--expect "wrote /LOGS/SYSREPORT.TXT" \
		--expect "[boot] desktop ready"

smoke-phase76-dynlink-deps: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "exec /bin/lddemo;;abi;;engine requirements;;engine log;;sysreport write;;cat /LOGS/SYSREPORT.TXT;;cat /SDK/README.TXT;;cat /SDK/BROWSER_ENGINE_PORT.TXT" \
		--expect "lddemo: abi=14" \
		--expect "lddemo: phase75 ok" \
		--expect "lddemo: phase76 objects=2 deps=1" \
		--expect "lddemo: phase76 dep module=1 tls=16" \
		--expect "lddemo: phase76 result=72 tls=23" \
		--expect "lddemo: phase76 ok" \
		--expect "coolOS-userspace-abi version 14" \
		--expect "file_backed_pages=6" \
		--expect "req.dynamic-linker=partial" \
		--expect "dynamic_linker=partial-file-mmap" \
		--expect "dynlink::load_with_deps" \
		--expect "wrote /LOGS/SYSREPORT.TXT" \
		--expect "[boot] desktop ready"

smoke-phase77-file-mmap: build
	python3 $(CURDIR)/scripts/qemu_smoke.py \
		--artifact-dir "$(SMOKE_ARTIFACT_DIR)" \
		--artifact-name "$@" \
		--bios "$(BIOS)" \
		--fsimg "$(FSIMG)" \
		--usb \
		--seconds $(SMOKE_FRAMEBUFFER_SECONDS) \
		--fw-cmd "exec /bin/mmapdemo;;exec /bin/lddemo;;abi;;diagnostics;;engine requirements;;engine log;;sysreport write;;cat /LOGS/SYSREPORT.TXT;;cat /SDK/README.TXT;;cat /SDK/BROWSER_ENGINE_PORT.TXT" \
		--expect "mmapdemo: abi=14" \
		--expect "mmapdemo: tmp roundtrip ok" \
		--expect "mmapdemo: file map /bin/motd.txt" \
		--expect "mmapdemo: mapped text coolOS Phase" \
		--expect "mmapdemo: write-map denied" \
		--expect "mmapdemo: exec-map ok" \
		--expect "mmapdemo: phase77 ok" \
		--expect "lddemo: phase76 ok" \
		--expect "coolOS-userspace-abi version 14" \
		--expect "file_backed_pages=9" \
		--expect "mmap_file" \
		--expect "file_mmap=partial-readonly" \
		--expect "dynamic_linker=partial-file-mmap" \
		--expect "wrote /LOGS/SYSREPORT.TXT" \
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
	(cd disk-image && cargo run --bin phase75-dso -- "$(PHASE75_DSO_TARGET)")
	(cd disk-image && cargo run --bin phase76-dsos -- "$(PHASE76_DEP_DSO_TARGET)" "$(PHASE76_MAIN_DSO_TARGET)")
	(cd disk-image && cargo run --bin disk-image -- "$(KERNEL)")
	(cd disk-image && cargo run --bin fs-image -- "$(FSIMG)" "$(USER_TARGET)" "$(USER_EXEC_TARGET)" "$(USER_PIPE_TARGET)" "$(USER_READ_TARGET)" "$(USER_PIPERD_TARGET)" "$(USER_PIPEWR_TARGET)" "$(USER_KEYECHO_TARGET)" "$(USER_TERMINAL_TARGET)" "$(USER_TTYREAD_TARGET)" "$(USER_NETDEMO_TARGET)" "$(USER_WGET_TARGET)" "$(USER_SDKDEMO_TARGET)" "$(USER_GUIDEMO_TARGET)" "$(USER_NOTES_TARGET)" "$(USER_EDITOR_TARGET)" "$(USER_TRASH_TARGET)" "$(USER_SCREENSHOT_TARGET)" "$(USER_PROCDEMO_TARGET)" "$(USER_PROCSLEEP_TARGET)" "$(USER_SENTINEL_TARGET)" "$(USER_BADPTR_TARGET)" "$(USER_BADWRITE_TARGET)" "$(USER_BADMMAP_TARGET)" "$(USER_BADEXEC_TARGET)" "$(USER_BADUSERREAD_TARGET)" "$(USER_SH_TARGET)" "$(USER_LS_TARGET)" "$(USER_CAT_TARGET)" "$(USER_ECHO_TARGET)" "$(USER_PWD_TARGET)" "$(USER_MKDIR_TARGET)" "$(USER_TOUCH_TARGET)" "$(USER_RM_TARGET)" "$(USER_WRITEFILE_TARGET)" "$(USER_CP_TARGET)" "$(USER_MV_TARGET)" "$(USER_GREP_TARGET)" "$(USER_HEAD_TARGET)" "$(USER_TAIL_TARGET)" "$(USER_DATE_TARGET)" "$(USER_UNAME_TARGET)" "$(USER_CLEAR_TARGET)" "$(USER_STAT_TARGET)" "$(USER_SYNC_TARGET)" "$(USER_DEVKIT_TARGET)" "$(USER_POLLDEMO_TARGET)" "$(USER_TUIDEMO_TARGET)" "$(USER_THREADDEMO_TARGET)" "$(USER_TLSDEMO_TARGET)" "$(USER_PTHREADDEMO_TARGET)" "$(USER_MMAPDEMO_TARGET)" "$(USER_LDDEMO_TARGET)" "$(PHASE75_DSO_TARGET)" "$(PHASE76_DEP_DSO_TARGET)" "$(PHASE76_MAIN_DSO_TARGET)")

build-uefi: build
	(cd disk-image && cargo run --features uefi --bin disk-image -- "$(KERNEL)")

build-uefi-safe: build
	(cd disk-image && COOLOS_IMAGE_SUFFIX="-safe" COOLOS_FB_WIDTH="$(USB_SAFE_FB_WIDTH)" COOLOS_FB_HEIGHT="$(USB_SAFE_FB_HEIGHT)" cargo run --features uefi --bin disk-image -- "$(KERNEL)")

build-secure-boot-keys:
	python3 scripts/secure_boot_artifacts.py keys \
		--out "$(SECURE_BOOT_DIR)" \
		--vars-template "$(QEMU_EFI_VARS_TEMPLATE)" \
		--secure-code "$(QEMU_EFI_SECURE_CODE)" \
		--pydeps "$(SECURE_BOOT_PYDEPS)"

build-uefi-secure: build build-secure-boot-keys
	(cd disk-image && COOLOS_IMAGE_SUFFIX="-secure" COOLOS_KERNEL_SHA256="$$(python3 ../scripts/secure_boot_artifacts.py kernel-hash "$(KERNEL)")" COOLOS_SIGN_EFI_LOADER=1 COOLOS_SECURE_BOOT_DIR="$(SECURE_BOOT_DIR)" COOLOS_SECURE_BOOT_SCRIPT="$(CURDIR)/scripts/secure_boot_artifacts.py" cargo run --features uefi --bin disk-image -- "$(KERNEL)")

build-uefi-secure-loader-tamper: build build-secure-boot-keys
	(cd disk-image && COOLOS_IMAGE_SUFFIX="-secure-loader-tamper" COOLOS_KERNEL_SHA256="$$(python3 ../scripts/secure_boot_artifacts.py kernel-hash "$(KERNEL)")" COOLOS_SIGN_EFI_LOADER=1 COOLOS_TAMPER_SIGNED_EFI_LOADER=1 COOLOS_SECURE_BOOT_DIR="$(SECURE_BOOT_DIR)" COOLOS_SECURE_BOOT_SCRIPT="$(CURDIR)/scripts/secure_boot_artifacts.py" cargo run --features uefi --bin disk-image -- "$(KERNEL)")

build-uefi-secure-kernel-tamper: build build-secure-boot-keys
	(cd disk-image && COOLOS_IMAGE_SUFFIX="-secure-kernel-tamper" COOLOS_KERNEL_SHA256="0000000000000000000000000000000000000000000000000000000000000000" COOLOS_SIGN_EFI_LOADER=1 COOLOS_SECURE_BOOT_DIR="$(SECURE_BOOT_DIR)" COOLOS_SECURE_BOOT_SCRIPT="$(CURDIR)/scripts/secure_boot_artifacts.py" cargo run --features uefi --bin disk-image -- "$(KERNEL)")

build-usb-image: build-uefi
	(cd disk-image && cargo run --bin usb_image -- "$(UEFI)" "$(FSIMG)" "$(USB_IMAGE)" "$(USB_IMAGE_SIZE_MIB)")

build-usb-safe-image: build-uefi-safe
	(cd disk-image && cargo run --bin usb_image -- "$(UEFI_SAFE)" "$(FSIMG)" "$(USB_SAFE_IMAGE)" "$(USB_IMAGE_SIZE_MIB)")

build-usb-secure-image: build-uefi-secure
	(cd disk-image && cargo run --bin usb_image -- "$(UEFI_SECURE)" "$(FSIMG)" "$(USB_SECURE_IMAGE)" "$(USB_IMAGE_SIZE_MIB)")

build-usb-secure-loader-tamper-image: build-uefi-secure-loader-tamper
	(cd disk-image && cargo run --bin usb_image -- "$(UEFI_SECURE_LOADER_TAMPER)" "$(FSIMG)" "$(USB_SECURE_LOADER_TAMPER_IMAGE)" "$(USB_IMAGE_SIZE_MIB)")

build-usb-secure-kernel-tamper-image: build-uefi-secure-kernel-tamper
	(cd disk-image && cargo run --bin usb_image -- "$(UEFI_SECURE_KERNEL_TAMPER)" "$(FSIMG)" "$(USB_SECURE_KERNEL_TAMPER_IMAGE)" "$(USB_IMAGE_SIZE_MIB)")

verify-secure-boot-artifacts: build-usb-secure-image
	python3 scripts/secure_boot_artifacts.py verify \
		--dir "$(SECURE_BOOT_DIR)" \
		--loader "$(SECURE_BOOT_DIR)/BOOTX64.EFI.signed" \
		--vars "$(QEMU_EFI_SECURE_VARS)" \
		--pydeps "$(SECURE_BOOT_PYDEPS)"

tamper-secure-boot-artifacts: build-usb-secure-loader-tamper-image build-usb-secure-kernel-tamper-image

build-usb-init: build

clean:
	cargo clean
	rm -rf target
	rm -rf disk-image/target
