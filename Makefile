#
# Makefile - Top-level build, clean and flash for the Rust demo application
#
# Author:  Sandesh Ghimire <sandesh@soccentric.com>
# Target:  NUCLEO-F767ZI (STM32F767ZI)
#
# Everything runs in the stm32-dev-rust container (RCON-001), the same way as
# scripts/build.sh. Flashing and debugging use the same OpenOCD set-up as the
# C projects.
#
#   make                build the firmware (same as make build)
#   make build          cargo build --release, then .elf/.bin/.hex/.img in build/
#   make clean          remove build/ and target/
#   make rebuild        clean, then build
#   make lint           cargo clippy -D warnings (RGEN-007)
#   make fmt            cargo fmt
#   make flash          build, then upload build/embassy-app.img through BOOT
#   make erase-all      erase the whole flash (asks for confirmation)
#   make reset          reset the board
#   make debug          build, flash, then GDB halted at reset
#   make gdbserver      OpenOCD GDB server on :3333
#   make serial         serial console on the ST-LINK VCP (115200 8N1)
#   make probe          detect the ST-LINK and the target chip
#   make size           show the firmware size
#   make image          build the stm32-dev-rust Docker image
#   make help           show this help
#

# Windows: run every recipe under Git's POSIX sh, also from cmd.exe or
# PowerShell with only Git\cmd on PATH. Otherwise native make falls back to
# cmd.exe, which cannot run the recipes (quotes, pipes, find, rm -rf), and
# `sh scripts/...` is not found. Git's usr\bin goes first on PATH so find,
# sort and friends are the POSIX tools, not the Windows ones of the same name.
ifeq ($(OS),Windows_NT)
ifneq ($(origin SHELL),command line)
GIT_USR_BIN := $(firstword \
  $(if $(wildcard C:/Program?Files/Git/usr/bin/sh.exe),C:/Program?Files/Git/usr/bin) \
  $(if $(wildcard $(subst \,/,$(LOCALAPPDATA))/Programs/Git/usr/bin/sh.exe),$(subst \,/,$(LOCALAPPDATA))/Programs/Git/usr/bin) \
  $(if $(wildcard C:/msys64/usr/bin/sh.exe),C:/msys64/usr/bin))
ifneq ($(GIT_USR_BIN),)
SHELL := $(subst ?, ,$(GIT_USR_BIN))/sh.exe
export PATH := $(subst /,\,$(subst ?, ,$(GIT_USR_BIN)));$(PATH)
endif
endif
endif

ELF      := build/embassy-app.elf
IMAGE    := build/embassy-app.img
# Gow make on Windows runs a simple recipe line itself instead of through a
# shell, and the MSYS bash.exe it starts that way exits 1 without running the
# script at all - no output, no status.  Prefixing the command with `sh` (Git
# Bash's own sh) takes the path make needs, so the script runs and its exit
# status is kept.  Linux and macOS start the scripts directly, as before.
ifeq ($(OS),Windows_NT)
DEV       := sh scripts/docker-dev.sh
BUILD_CMD := sh scripts/build.sh
else
DEV       := scripts/docker-dev.sh
BUILD_CMD := scripts/build.sh
endif
CARGO    := $(DEV) run env CARGO_HOME=/work/.cargo-home cargo
BOOTCTL  ?= $(firstword $(wildcard ../bootloader/tools/bootctl.py ../../bootloader/bootloader/tools/bootctl.py) ../bootloader/tools/bootctl.py)
BOOT_DIR   ?= $(patsubst %/tools/bootctl.py,%,$(BOOTCTL))
SIGN_KEY   ?= $(BOOT_DIR)/keys/dev-signing-key.pem
SECURITY_COUNTER ?= 3
SIGNED     := $(IMAGE:.img=-signed.img)
PORT     ?=
# Git Bash resolves python3 to the Microsoft Store stub, which exits non-zero;
# Windows has the interpreter as python.
PYTHON   ?= $(if $(filter Windows_NT,$(OS)),python,python3)

.DEFAULT_GOAL := help

.PHONY: all build clean rebuild lint fmt flash erase-all reset debug \
        gdbserver serial probe size image help

all: build

build:
	$(BUILD_CMD)

clean:
	rm -rf build target

rebuild: clean
	@"$(MAKE)" --no-print-directory build

lint:
	$(CARGO) clippy --release -- -D warnings

fmt:
	$(CARGO) fmt

# A-6: never program the raw ELF into a slot. BOOT only starts an image
# that has its 256-byte header, metadata and records, which bootctl writes.
flash: build
	@test -f $(IMAGE) || { echo "error: $(IMAGE) not found, run make build"; exit 1; }
	@test -f $(BOOTCTL) || { echo "error: $(BOOTCTL) not found; set BOOTCTL=/path/to/bootctl.py"; exit 1; }
	@test -f $(SIGN_KEY) || { echo "error: $(SIGN_KEY) not found; set SIGN_KEY=/path/to/key.pem"; exit 1; }
	$(PYTHON) $(BOOT_DIR)/tools/mkimage.py --sign $(IMAGE) --key $(SIGN_KEY) \
		--security-counter $(SECURITY_COUNTER) --output $(SIGNED)
	$(PYTHON) $(BOOTCTL) $(if $(PORT),--port $(PORT),) update $(SIGNED) --activate

erase-all:
	@echo "WARNING: this erases the whole flash: BOOT, both metadata sectors"
	@echo "         and both application slots."
	@printf "Type 'yes' to continue: " && read answer && [ "$$answer" = "yes" ]
	$(DEV) erase

reset:
	$(DEV) reset

debug: build
	$(DEV) debug $(ELF) --load

gdbserver:
	$(DEV) gdbserver

serial:
	$(DEV) serial

probe:
	$(DEV) probe

size:
	@test -f $(ELF) || { echo "error: $(ELF) not found, run make build"; exit 1; }
	$(DEV) run arm-none-eabi-size /work/$(ELF)

image:
	$(DEV) build

help:
	@sed -n "/^#   make/p" $(firstword $(MAKEFILE_LIST)) | sed "s/^# \{0,1\}//"
