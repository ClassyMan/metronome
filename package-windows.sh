#!/bin/bash
set -euo pipefail

SYSROOT="$HOME/msys2-sysroot/mingw64"
EXE="target/x86_64-pc-windows-gnu/release/metronome.exe"
DIST="dist/metronome-windows"
GRESOURCE="builddir/data/resources/resources.gresource"

if [ ! -f "$EXE" ]; then
    echo "ERROR: $EXE not found. Run the cross-compile first."
    exit 1
fi

if [ ! -f "$GRESOURCE" ]; then
    echo "ERROR: $GRESOURCE not found. Run 'ninja -C builddir' first."
    exit 1
fi

rm -rf "$DIST"
mkdir -p "$DIST/lib/gstreamer-1.0"
mkdir -p "$DIST/lib/gdk-pixbuf-2.0/2.10.0/loaders"
mkdir -p "$DIST/share/glib-2.0/schemas"
mkdir -p "$DIST/share/icons"
mkdir -p "$DIST/themes"

echo "=== Copying exe and resources ==="
cp "$EXE" "$DIST/metronome.exe"
cp "$GRESOURCE" "$DIST/resources.gresource"

echo "=== Copying background image ==="
cp /home/aidany/Pictures/icaro_paiva_guitarrista-png.webp "$DIST/background.webp"

echo "=== Writing config.json ==="
cat > "$DIST/config.json" << 'EOJSON'
{
  "beats-per-bar": 1,
  "beats-per-minute": 133,
  "active-theme": "Catppuccin Mocha",
  "tempo-ramp-enabled": true,
  "tempo-ramp-increment": 5,
  "tempo-ramp-bars": 20,
  "tempo-ramp-target": 260,
  "background-image-path": "background.webp",
  "background-opacity": 0.27,
  "background-style": "cover",
  "window-width": -1,
  "window-height": -1,
  "is-maximized": false
}
EOJSON

echo "=== Copying user themes ==="
cp /home/aidany/.local/share/metronome/themes/*.json "$DIST/themes/" 2>/dev/null || true

echo "=== Resolving DLL dependencies ==="
copy_dll_deps() {
    local binary="$1"
    for dll in $(x86_64-w64-mingw32-objdump -p "$binary" 2>/dev/null | grep "DLL Name" | awk '{print $3}'); do
        local dll_lower=$(echo "$dll" | tr 'A-Z' 'a-z')
        case "$dll_lower" in
            kernel32.dll|user32.dll|gdi32.dll|advapi32.dll|shell32.dll|\
            ole32.dll|oleaut32.dll|ws2_32.dll|msvcrt.dll|ntdll.dll|\
            imm32.dll|comctl32.dll|comdlg32.dll|winspool.dll|winmm.dll|\
            shlwapi.dll|version.dll|setupapi.dll|cfgmgr32.dll|crypt32.dll|\
            bcrypt.dll|secur32.dll|mswsock.dll|dnsapi.dll|iphlpapi.dll|\
            psapi.dll|ucrtbase.dll|dwmapi.dll|dxgi.dll|opengl32.dll|\
            userenv.dll|hid.dll|wintrust.dll|rpcrt4.dll|sspicli.dll|\
            uxtheme.dll|powrprof.dll|dbghelp.dll|mfplat.dll|mfreadwrite.dll|\
            propsys.dll|ksuser.dll|avrt.dll|d3d11.dll|d3d10.dll|\
            d3d9.dll|dwrite.dll|d2d1.dll|windowscodecs.dll|msimg32.dll|\
            sechost.dll|ncrypt.dll|api-ms-*)
                continue ;;
        esac
        if [ -f "$SYSROOT/bin/$dll" ] && [ ! -f "$DIST/$dll" ]; then
            cp "$SYSROOT/bin/$dll" "$DIST/"
            copy_dll_deps "$DIST/$dll"
        fi
    done
}

copy_dll_deps "$DIST/metronome.exe"

echo "=== Copying GStreamer plugins ==="
for plugin in coreelements playback autodetect audioconvert audioresample \
              vorbis ogg typefindfunctions audioparsers gio app wasapi2; do
    src="$SYSROOT/lib/gstreamer-1.0/libgst${plugin}.dll"
    if [ -f "$src" ]; then
        cp "$src" "$DIST/lib/gstreamer-1.0/"
        copy_dll_deps "$src"
    else
        echo "  plugin not found: $plugin"
    fi
done

echo "=== Copying gdk-pixbuf loaders ==="
if [ -d "$SYSROOT/lib/gdk-pixbuf-2.0/2.10.0/loaders" ]; then
    for loader in png jpeg webp; do
        src=$(ls "$SYSROOT/lib/gdk-pixbuf-2.0/2.10.0/loaders/libpixbufloader-${loader}.dll" 2>/dev/null || true)
        if [ -n "$src" ]; then
            cp "$src" "$DIST/lib/gdk-pixbuf-2.0/2.10.0/loaders/"
        fi
    done
fi

echo "=== Copying GSettings schemas ==="
cp "$SYSROOT/share/glib-2.0/schemas/"*.gschema.xml "$DIST/share/glib-2.0/schemas/" 2>/dev/null || true
glib-compile-schemas "$DIST/share/glib-2.0/schemas/" 2>/dev/null || true

echo "=== Copying icon themes ==="
if [ -d "$SYSROOT/share/icons/hicolor" ]; then
    cp -r "$SYSROOT/share/icons/hicolor" "$DIST/share/icons/"
fi
if [ -d "$SYSROOT/share/icons/Adwaita" ]; then
    mkdir -p "$DIST/share/icons/Adwaita"
    cp -r "$SYSROOT/share/icons/Adwaita/symbolic" "$DIST/share/icons/Adwaita/" 2>/dev/null || true
    cp "$SYSROOT/share/icons/Adwaita/index.theme" "$DIST/share/icons/Adwaita/" 2>/dev/null || true
fi

echo "=== Stripping binaries ==="
x86_64-w64-mingw32-strip "$DIST/metronome.exe" 2>/dev/null || true
x86_64-w64-mingw32-strip "$DIST/"*.dll 2>/dev/null || true
x86_64-w64-mingw32-strip "$DIST/lib/gstreamer-1.0/"*.dll 2>/dev/null || true

echo "=== Creating zip ==="
cd dist
zip -9 -r metronome-windows.zip metronome-windows/
cd ..

echo ""
echo "=== Done ==="
ls -lh dist/metronome-windows.zip
echo "DLL count: $(ls "$DIST/"*.dll 2>/dev/null | wc -l)"
echo "GStreamer plugins: $(ls "$DIST/lib/gstreamer-1.0/"*.dll 2>/dev/null | wc -l)"
echo "Total size (uncompressed): $(du -sh "$DIST" | cut -f1)"
