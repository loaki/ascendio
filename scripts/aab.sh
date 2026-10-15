#!/bin/sh
# Repackage the APK cargo-quad-apk builds as the Android App Bundle Play
# wants, then sign it with the upload key. Runs inside the ascendio-apk image
# (`make aab`): aapt2 rewrites the binary manifest/resources in the protobuf
# format bundles use, the files move into bundletool's module layout, and
# bundletool wraps that module as the bundle.
#
#   aab.sh <apk> <aab> <keystore>
#
# The keystore password comes from $UPLOAD_STORE_PASS, or jarsigner asks.
set -eu

apk=$1 aab=$2 keystore=$3
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

aapt2=$(ls "$ANDROID_HOME"/build-tools/*/aapt2 | tail -n 1)
"$aapt2" convert --output-format proto -o "$work/proto.apk" "$apk"

mkdir "$work/base"
cd "$work/base"
unzip -q ../proto.apk
# The APK's debug signature means nothing inside a bundle.
rm -rf META-INF
mkdir manifest dex root
mv AndroidManifest.xml manifest/
for f in *.dex; do [ -e "$f" ] && mv "$f" dex/; done
for f in *; do
    case $f in
        manifest | dex | res | lib | assets | resources.pb | root) ;;
        *) mv "$f" root/ ;;
    esac
done
bsdtar --format zip -cf ../base.zip *
cd - >/dev/null

rm -f "$aab"
java -jar /opt/bundletool.jar build-bundle --modules="$work/base.zip" --output="$aab"

if [ -n "${UPLOAD_STORE_PASS:-}" ]; then
    jarsigner -sigalg SHA256withRSA -digestalg SHA-256 -storepass:env UPLOAD_STORE_PASS \
        -keystore "$keystore" "$aab" upload >/dev/null
else
    jarsigner -sigalg SHA256withRSA -digestalg SHA-256 -keystore "$keystore" "$aab" upload >/dev/null
fi
echo "AAB -> $aab"
