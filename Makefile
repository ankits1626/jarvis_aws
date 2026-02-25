.PHONY: sidecar app install clean dev dmg

APP_NAME := JarvisApp
SIDECAR_NAME := JarvisListen
BUNDLE_PATH := jarvis-app/src-tauri/target/release/bundle/macos/$(APP_NAME).app
DMG_DIR := jarvis-app/src-tauri/target/release/bundle/dmg
DMG_PATH := $(DMG_DIR)/$(APP_NAME)_0.1.0_aarch64.dmg
INSTALL_PATH := /Applications/$(APP_NAME).app
SIDECAR_SRC := jarvis-listen
SIDECAR_BIN := $(SIDECAR_SRC)/.build/release/$(SIDECAR_NAME)
SIDECAR_DEST := jarvis-app/src-tauri/binaries/$(SIDECAR_NAME)-aarch64-apple-darwin
ENTITLEMENTS := jarvis-app/src-tauri/entitlements.plist

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
	@echo "==> Signing with hardened runtime..."
	# Sign inner components first (inside-out signing)
	codesign --force --sign - --options runtime "$(BUNDLE_PATH)/Contents/Frameworks/libvosk.dylib"
	codesign --force --sign - --options runtime "$(BUNDLE_PATH)/Contents/MacOS/JarvisListen"
	codesign --force --sign - --options runtime "$(BUNDLE_PATH)/Contents/MacOS/IntelligenceKit"
	# Sign the main app bundle with entitlements
	codesign --force --sign - --options runtime --entitlements "$(ENTITLEMENTS)" "$(BUNDLE_PATH)"
	codesign --verify --strict "$(BUNDLE_PATH)"
	@echo "==> App built and signed with hardened runtime"

# Rebuild DMG from the signed .app (Tauri's DMG is built before our signing)
dmg: app
	@echo "==> Rebuilding DMG with signed app..."
	rm -f "$(DMG_PATH)"
	hdiutil create -volname "$(APP_NAME)" -srcfolder "$(BUNDLE_PATH)" -ov -format UDZO "$(DMG_PATH)"
	xattr -cr "$(DMG_PATH)"
	@echo "==> DMG ready: $(DMG_PATH)"

# Install to /Applications
install: app
	@echo "==> Installing to $(INSTALL_PATH)..."
	rm -rf "$(INSTALL_PATH)"
	cp -R "$(BUNDLE_PATH)" "$(INSTALL_PATH)"
	# Remove quarantine flag so macOS doesn't cache a denied permission state on first launch
	xattr -cr "$(INSTALL_PATH)"
	# Reset TCC cache for this app so permission prompts appear fresh
	-tccutil reset All com.jarvis.app 2>/dev/null
	@echo "==> Installed. Launch $(APP_NAME) from /Applications"
	@echo ""
	@echo "NOTE: On first launch, grant permissions in"
	@echo "  System Settings > Privacy & Security for:"
	@echo "  - Microphone"
	@echo "  - Accessibility"
	@echo "  - Automation (Google Chrome)"

# Run in dev mode (no install needed)
dev: sidecar
	cd jarvis-app && npx tauri dev

# Clean build artifacts
clean:
	cd $(SIDECAR_SRC) && swift package clean
	cd jarvis-app && rm -rf src-tauri/target dist
	@echo "==> Cleaned"
