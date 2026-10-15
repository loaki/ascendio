.PHONY: run test lint fmt web serve ip apk install release upload-key aab clean

WEB_PORT ?= 8000

# See scripts/lan-ip.sh -- the address a phone on the LAN can actually reach,
# ignoring docker bridges and the VPN tunnel. Resolved once, at parse time.
LAN_IP := $(shell ./scripts/lan-ip.sh)
LAN_SUBNET := $(shell ./scripts/lan-ip.sh | cut -d. -f1-3).0/24

run:
	cargo run --release

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

# --- web (the fastest way to test on a phone) -------------------------------

web:
	cargo build --release --target wasm32-unknown-unknown
	cp target/wasm32-unknown-unknown/release/ascendio.wasm web/ascendio.wasm
	@echo "built web/ascendio.wasm ($$(du -h web/ascendio.wasm | cut -f1))"

# Build, then serve on the LAN so a phone on the same wifi can open it.
serve: web
	@echo ""
	@echo "  phone (same wifi):  http://$(LAN_IP):$(WEB_PORT)"
	@echo "  this machine:       http://localhost:$(WEB_PORT)"
	@echo ""
	@echo "  if the phone times out, ufw is dropping it -- allow the port:"
	@echo "    sudo ufw allow from $(LAN_SUBNET) to any port $(WEB_PORT) proto tcp"
	@echo ""
	@python3 -m http.server $(WEB_PORT) --bind 0.0.0.0 --directory web

ip:
	@echo "http://$(LAN_IP):$(WEB_PORT)"

# --- android ----------------------------------------------------------------

# One stamp per make run, shown in the game's top bar and used as the tag.
NOW := $(shell date +%s)
VERSION := v$(shell date -d @$(NOW) +%Y.%m.%d-%H%M)
# Android's versionCode: Play refuses an upload whose code isn't above the
# last one, and minutes since 1970 always is (well under Play's 2100000000).
VERSION_CODE := $(shell echo $$(($(NOW) / 60)))

# Android build via the macroquad-maintained toolchain image, with a current
# Rust on top (scripts/android.Dockerfile). Needs Docker. cargo-quad-apk
# embeds an old cargo, so Cargo.lock must stay at `version = 3`.
# .android/ keeps the debug signing key between builds: a new key would make
# the phone refuse the update (and uninstalling wipes the save).
# cargo-quad-apk only reads the version from Cargo.toml, so the build sees a
# copy with this run's version mounted over the real one.
# $(1): extra `docker run` flags.
define quad_apk
	docker build -q -t ascendio-apk -f scripts/android.Dockerfile scripts
	@mkdir -p target
	sed '/^\[package.metadata.android\]$$/a version_code = $(VERSION_CODE)\nversion_name = "$(VERSION)"' \
		Cargo.toml > target/android.Cargo.toml
	docker run --rm -v "$(PWD)":/root/src -v "$(PWD)/.android":/root/.android -w /root/src \
		-v "$(PWD)/target/android.Cargo.toml":/root/src/Cargo.toml:ro \
		-e ASCENDIO_VERSION=$(VERSION) $(1) \
		ascendio-apk cargo quad-apk build --release
endef

apk:
	$(call quad_apk)
	@echo "APK -> target/android-artifacts/release/apk/ascendio.apk"

# --- google play ------------------------------------------------------------

# The key Play knows uploads come from (Play re-signs for players with its
# own). Lose it and you need Play support to reset it -- back it up, with its
# password, somewhere other than this machine.
UPLOAD_KEYSTORE := .android/upload.keystore

upload-key:
	@test ! -e $(UPLOAD_KEYSTORE) || { echo "$(UPLOAD_KEYSTORE) already exists"; exit 1; }
	docker build -q -t ascendio-apk -f scripts/android.Dockerfile scripts
	docker run --rm -it -v "$(PWD)/.android":/root/.android ascendio-apk \
		keytool -genkeypair -keystore /root/$(UPLOAD_KEYSTORE) -alias upload \
		-keyalg RSA -keysize 4096 -validity 10000

# The bundle to upload to Play: a Play build (no GitHub update check) as an
# APK, repackaged by scripts/aab.sh. Signing asks for the keystore password,
# unless UPLOAD_STORE_PASS is set.
aab:
	@test -e $(UPLOAD_KEYSTORE) || { echo "no upload key: run 'make upload-key' first"; exit 1; }
	$(call quad_apk,-e ASCENDIO_STORE=play)
	docker run --rm $(if $(UPLOAD_STORE_PASS),-e UPLOAD_STORE_PASS,-it) \
		-v "$(PWD)":/root/src -v "$(PWD)/.android":/root/.android -w /root/src \
		ascendio-apk sh scripts/aab.sh target/android-artifacts/release/apk/ascendio.apk \
		target/ascendio.aab /root/$(UPLOAD_KEYSTORE)
	@echo "upload target/ascendio.aab ($(VERSION), versionCode $(VERSION_CODE))"

install: apk
	adb install -r target/android-artifacts/release/apk/ascendio.apk

# Publish the APK as a GitHub release. On the phone, the newest one is always at
#   https://github.com/loaki/ascendio/releases/latest/download/ascendio.apk
RELEASE_TAG ?= $(VERSION)
release: apk
	gh release create $(RELEASE_TAG) target/android-artifacts/release/apk/ascendio.apk \
		--title "$(RELEASE_TAG)" --notes "Android build of $$(git rev-parse --short HEAD)."

clean:
	cargo clean
