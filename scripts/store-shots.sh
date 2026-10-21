#!/bin/sh
# Google Play screenshots from the desktop build's shot mode, each after two
# minutes of autoplay (about half the tree found, never touching the save):
# 1080x1920 phone shots in store/phone/, 1440x2560 tablet shots in
# store/tablet/ (both 9:16). Needs a display; the windows run side by side.
set -e
cd "$(dirname "$0")/.."
cargo build --release
MODES="spiral dial map keystones detail biomes"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# $1: folder, $2 $3: window size
shots() {
    mkdir -p "$tmp/$1"
    for m in $MODES; do
        ASCENDIO_AUTOPLAY=1 ASCENDIO_W=$2 ASCENDIO_H=$3 ASCENDIO_SHOT="$tmp/$1/$m.png" \
            ASCENDIO_SHOT_MODE=$m ASCENDIO_SHOT_AFTER=120 target/release/ascendio &
    done
}

# $1: folder, $2 $3: final size
scale() {
    mkdir -p "store/$1"
    for m in $MODES; do
        # high_dpi may already have doubled it: scale to the final size,
        # by whole pixels when it divides.
        python3 - "$tmp/$1/$m.png" "store/$1/$m.png" "$2" "$3" <<'EOF'
import sys
from PIL import Image
src, dst, w, h = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
img = Image.open(src).convert("RGB")
sharp = w % img.width == 0 and h % img.height == 0
img.resize((w, h), Image.NEAREST if sharp else Image.LANCZOS).save(dst)
EOF
    done
}

shots phone 540 960
shots tablet 720 1280
wait
scale phone 1080 1920
scale tablet 1440 2560
echo "wrote store/phone/ and store/tablet/"
