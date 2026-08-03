#!/bin/bash
set -e

echo "=== Building PBS Soundpad AppImage ==="

# 1. Compile release binary
cargo build --release

# 2. Create build directories
BUILD_DIR="appimage_build"
APPDIR="${BUILD_DIR}/PBS.AppDir"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/48x48/apps"

# 3. Copy binary
cp target/release/pipewire-browser-soundpad "${APPDIR}/usr/bin/"

# 4. Copy system icon if available, otherwise fallback
ICON_SRC="/usr/share/icons/HighContrast/48x48/devices/audio-input-microphone.png"
if [ -f "$ICON_SRC" ]; then
    cp "$ICON_SRC" "${APPDIR}/pipewire-browser-soundpad.png"
else
    # Fallback to copy any available device icon or create an empty file if none exists
    find /usr/share/icons -name "*.png" | head -n 1 | xargs -I {} cp {} "${APPDIR}/pipewire-browser-soundpad.png"
fi
cp "${APPDIR}/pipewire-browser-soundpad.png" "${APPDIR}/usr/share/icons/hicolor/48x48/apps/pipewire-browser-soundpad.png"

# 5. Create Desktop Entry
cat > "${APPDIR}/pipewire-browser-soundpad.desktop" << 'EOF'
[Desktop Entry]
Type=Application
Name=PBS Soundpad
Comment=Virtual Microphone Router for browsers and physical mics
Exec=pipewire-browser-soundpad
Icon=pipewire-browser-soundpad
Categories=AudioVideo;Audio;
Terminal=false
EOF
cp "${APPDIR}/pipewire-browser-soundpad.desktop" "${APPDIR}/usr/share/applications/"

# 6. Create AppRun script
cat > "${APPDIR}/AppRun" << 'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
exec "${HERE}/usr/bin/pipewire-browser-soundpad" "$@"
EOF
chmod +x "${APPDIR}/AppRun"

# 7. Download appimagetool if not present
if [ ! -d "${BUILD_DIR}/squashfs-root" ]; then
    echo "Downloading appimagetool..."
    rm -f "${BUILD_DIR}/appimagetool"
    curl -Lo "${BUILD_DIR}/appimagetool" "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
    chmod +x "${BUILD_DIR}/appimagetool"
    
    echo "Extracting appimagetool..."
    cd "${BUILD_DIR}"
    ./appimagetool --appimage-extract
    cd ..
fi

# 8. Pack AppImage using extracted appimagetool
echo "Generating AppImage..."
# ARCH is required by appimagetool
export ARCH=x86_64
"${BUILD_DIR}/squashfs-root/AppRun" "${APPDIR}" "PBS_Soundpad-x86_64.AppImage"

echo "=== AppImage Created Successfully! ==="
ls -lh "PBS_Soundpad-x86_64.AppImage"
