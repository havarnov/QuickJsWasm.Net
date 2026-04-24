PROJECT_NAME := quickjs-wasm-dotnet

# 1. ARCHITECTURE DETECTION
# Get the Operating System (e.g., Darwin, Linux)
OS := $(shell uname -s)
# Get the Machine Architecture (e.g., x86_64, aarch64)
ARCH := $(shell uname -m)

# 2. Determine Library Extension and Destination RID
# Initialize variables
NATIVE_LIB_NAME := libhost
DEST_NATIVE_LIB_NAME := libhost_dotnet
NATIVE_LIB_EXT :=
RID :=

ifeq ($(OS),Darwin) # macOS
    NATIVE_LIB_EXT := .dylib
    ifeq ($(ARCH),x86_64)
        RID := osx-x64
    else ifeq ($(ARCH),arm64)
        RID := osx-arm64
    endif
else ifeq ($(OS),Linux) # Linux
    NATIVE_LIB_EXT := .so
    ifeq ($(ARCH),x86_64)
        RID := linux-x64
    else ifeq ($(ARCH),aarch64)
        RID := linux-arm64
    endif
else
    # Fallback or error for unsupported systems
    $(error Unsupported OS/ARCH combination: $(OS)/$(ARCH). Update Makefile.)
endif

# Construct the full file paths
SOURCE_LIB_PATH := ./target/release/$(NATIVE_LIB_NAME)$(NATIVE_LIB_EXT)
DEST_DIR := QuickJsWasm.Net/runtimes/$(RID)/native
DEST_LIB_PATH := $(DEST_DIR)/$(DEST_NATIVE_LIB_NAME)$(NATIVE_LIB_EXT)

all: build

build: copy

build-guest:
	cargo build --release --target wasm32-wasip2 --manifest-path guest/Cargo.toml

build-host: build-guest
	cargo build --release --manifest-path host/Cargo.toml

copy: build-host
	@echo "Copying native library for $(RID) to $(DEST_DIR)/"
	mkdir -p $(DEST_DIR)
	cp $(SOURCE_LIB_PATH) $(DEST_LIB_PATH)
	cp target/wasm32-wasip2/release/guest.wasm QuickJsWasm.Net/

# Phony targets to prevent conflicts with files of the same name
.PHONY: all build-native copy
