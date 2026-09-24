.PHONY: run test lint fmt web serve ip apk install release clean

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

# Android build via the macroquad-maintained toolchain image, with a current
# Rust on top (scripts/android.Dockerfile). Needs Docker. cargo-quad-apk
# embeds an old cargo, so Cargo.lock must stay at `version = 3`.
# .android/ keeps the debug signing key between builds: a new key would make
# the phone refuse the update (and uninstalling wipes the save).
apk:
	docker build -q -t ascendio-apk -f scripts/android.Dockerfile scripts
	docker run --rm -v "$(PWD)":/root/src -v "$(PWD)/.android":/root/.android -w /root/src \
		ascendio-apk cargo quad-apk build --release
	@echo "APK -> target/android-artifacts/release/apk/ascendio.apk"

install: apk
	adb install -r target/android-artifacts/release/apk/ascendio.apk

# Publish the APK as a GitHub release. On the phone, the newest one is always at
#   https://github.com/loaki/ascendio/releases/latest/download/ascendio.apk
RELEASE_TAG ?= v$(shell date +%Y.%m.%d-%H%M)
release: apk
	gh release create $(RELEASE_TAG) target/android-artifacts/release/apk/ascendio.apk \
		--title "$(RELEASE_TAG)" --notes "Android build of $$(git rev-parse --short HEAD)."

clean:
	cargo clean
