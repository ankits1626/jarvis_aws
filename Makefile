.PHONY: sidecar app install clean dev

APP_NAME := JarvisApp
SIDECAR_NAME := JarvisListen
BUNDLE_PATH := jarvis-app/src-tauri/target/release/bundle/macos/$(APP_NAME).app
INSTALL_PATH := /Applications/$(APP_NAME).app
SIDECAR_SRC := jarvis-listen
SIDECAR_BIN := $(SIDECAR_SRC)/.build/release/$(SIDECAR_NAME)
SIDECAR_DEST := jarvis-app/src-tauri/binaries/$(SIDECAR_NAME)-aarch64-apple-darwin

# Build everything, sign, and install
all: install

# Build the Swift sidecar
sidecar:
	@echo "==> Building sidecar..."
	cd $(SIDECAR_SRC) && swift build -c release
	cp $(SIDECAR_BIN) $(SIDECAR_DEST)
	@echo "==> Sidecar built and copied"

# Build the Tauri app (includes sidecar)
app: sidecar
	@echo "==> Building Tauri app..."
	cd jarvis-app && npx tauri build
	@echo "==> Fixing code signature..."
	codesign --force --deep --sign - "$(BUNDLE_PATH)"
	codesign --verify --deep --strict "$(BUNDLE_PATH)"
	@echo "==> App built and signed"

# Install to /Applications
install: app
	@echo "==> Installing to $(INSTALL_PATH)..."
	rm -rf "$(INSTALL_PATH)"
	cp -R "$(BUNDLE_PATH)" "$(INSTALL_PATH)"
	@echo "==> Installed. Launch $(APP_NAME) from /Applications"

# Run in dev mode (no install needed)
dev: sidecar
	cd jarvis-app && npx tauri dev

# Clean build artifacts
clean:
	cd $(SIDECAR_SRC) && swift package clean
	cd jarvis-app && rm -rf src-tauri/target dist
	@echo "==> Cleaned"
