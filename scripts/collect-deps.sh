#!/bin/bash
# Collect external dependencies into src-tauri/resources/ for bundling.
# Run this before `cargo tauri build`.
#
# Usage: ./scripts/collect-deps.sh [macos|windows]
#   - Auto-detects platform from `uname -s` if no arg given.
#   - Windows is detected from Git Bash / MSYS (mingw*, msys*).

set -euo pipefail

RAW_PLATFORM="${1:-$(uname -s | tr '[:upper:]' '[:lower:]')}"
case "$RAW_PLATFORM" in
    darwin|macos) PLATFORM="macos" ;;
    mingw*|msys*|cygwin*|windows*) PLATFORM="windows" ;;
    linux*) PLATFORM="linux" ;;
    *) PLATFORM="$RAW_PLATFORM" ;;
esac

RESOURCES="src-tauri/resources"
SCRCPY_VERSION="3.3.4"
APKTOOL_VERSION="2.11.1"

mkdir -p "$RESOURCES/bin"

echo "=== Collecting dependencies for platform: $PLATFORM ==="

# ---------------------------------------------------------------------------
# macOS branch — reuses locally installed Homebrew / Android SDK tools.
# ---------------------------------------------------------------------------
collect_macos() {
    # --- scrcpy-server ---
    echo "[1/5] scrcpy-server..."
    if [ -f /opt/homebrew/share/scrcpy/scrcpy-server ]; then
        cp /opt/homebrew/share/scrcpy/scrcpy-server "$RESOURCES/scrcpy-server.jar"
        echo "  -> Copied from Homebrew"
    elif [ -f /usr/local/share/scrcpy/scrcpy-server ]; then
        cp /usr/local/share/scrcpy/scrcpy-server "$RESOURCES/scrcpy-server.jar"
        echo "  -> Copied from /usr/local"
    elif [ -f "$RESOURCES/scrcpy-server.jar" ]; then
        echo "  -> Already present"
    else
        echo "  !! scrcpy-server not found. Install scrcpy or manually copy scrcpy-server to $RESOURCES/scrcpy-server.jar"
    fi

    # --- adb ---
    echo "[2/5] adb..."
    ADB_PATH=$(which adb 2>/dev/null || true)
    if [ -n "$ADB_PATH" ]; then
        cp "$ADB_PATH" "$RESOURCES/bin/adb"
        chmod +x "$RESOURCES/bin/adb"
        echo "  -> Copied from $ADB_PATH"
    elif [ -f "$RESOURCES/bin/adb" ]; then
        echo "  -> Already present"
    else
        echo "  !! adb not found. Install Android platform-tools."
    fi

    # --- apktool ---
    echo "[3/5] apktool..."
    APKTOOL_JAR=""
    for dir in /opt/homebrew/opt/apktool/libexec /opt/homebrew/Cellar/apktool/*/libexec /usr/local/opt/apktool/libexec; do
        if [ -f "$dir/apktool.jar" ]; then
            APKTOOL_JAR="$dir/apktool.jar"
            break
        fi
        FOUND=$(ls "$dir"/apktool*.jar 2>/dev/null | head -1)
        if [ -n "$FOUND" ]; then
            APKTOOL_JAR="$FOUND"
            break
        fi
    done

    if [ -n "$APKTOOL_JAR" ]; then
        cp "$APKTOOL_JAR" "$RESOURCES/apktool.jar"
        echo "  -> Copied from $APKTOOL_JAR"
    elif [ -f "$RESOURCES/apktool.jar" ]; then
        echo "  -> Already present"
    else
        echo "  !! apktool.jar not found. Install: brew install apktool"
    fi

    # --- apksigner + zipalign (from Android SDK) ---
    echo "[4/5] apksigner + zipalign..."
    APKSIGNER_JAR=""
    ZIPALIGN_PATH=""
    if [ -d "$HOME/Library/Android/sdk/build-tools" ]; then
        LATEST=$(ls -1 "$HOME/Library/Android/sdk/build-tools" | sort -V | tail -1)
        if [ -f "$HOME/Library/Android/sdk/build-tools/$LATEST/lib/apksigner.jar" ]; then
            APKSIGNER_JAR="$HOME/Library/Android/sdk/build-tools/$LATEST/lib/apksigner.jar"
        fi
        ZIPALIGN_PATH="$HOME/Library/Android/sdk/build-tools/$LATEST/zipalign"
    fi

    if [ -n "$APKSIGNER_JAR" ]; then
        cp "$APKSIGNER_JAR" "$RESOURCES/apksigner.jar"
        echo "  -> apksigner copied from $APKSIGNER_JAR"
    elif [ -f "$RESOURCES/apksigner.jar" ]; then
        echo "  -> apksigner already present"
    else
        echo "  !! apksigner.jar not found. Install Android SDK build-tools."
        echo "     Fallback: jarsigner (from JDK) will be used instead."
    fi

    if [ -n "$ZIPALIGN_PATH" ] && [ -f "$ZIPALIGN_PATH" ]; then
        cp "$ZIPALIGN_PATH" "$RESOURCES/bin/zipalign"
        chmod +x "$RESOURCES/bin/zipalign"
        echo "  -> zipalign copied from $ZIPALIGN_PATH"
    fi
}

# ---------------------------------------------------------------------------
# Windows branch — downloads artefacts exactly like .github/workflows/build.yml
# does on windows-latest. Runs from Git Bash / MSYS.
# Requires: curl, unzip, and JDK 21 in PATH (for jlink).
# ---------------------------------------------------------------------------
collect_windows() {
    local TMPDIR="${TMP:-/tmp}"
    mkdir -p "$TMPDIR"

    # --- scrcpy-server ---
    echo "[1/5] scrcpy-server..."
    if [ -f "$RESOURCES/scrcpy-server.jar" ]; then
        echo "  -> Already present"
    else
        # Official scrcpy releases (Genymobile). Asset is named scrcpy-server-v<ver>.
        if curl -fL "https://github.com/Genymobile/scrcpy/releases/download/v${SCRCPY_VERSION}/scrcpy-server-v${SCRCPY_VERSION}" \
            -o "$RESOURCES/scrcpy-server.jar"; then
            echo "  -> Downloaded scrcpy-server v${SCRCPY_VERSION} (Genymobile)"
        else
            rm -f "$RESOURCES/scrcpy-server.jar"
            echo "  !! Download failed. Place scrcpy-server manually in $RESOURCES/scrcpy-server.jar"
            echo "     Direct URL: https://github.com/Genymobile/scrcpy/releases/tag/v${SCRCPY_VERSION}"
        fi
    fi

    # --- adb.exe + DLLs ---
    echo "[2/5] adb (Windows platform-tools)..."
    if [ -f "$RESOURCES/bin/adb.exe" ] && [ -f "$RESOURCES/bin/AdbWinApi.dll" ]; then
        echo "  -> Already present"
    else
        curl -fL "https://dl.google.com/android/repository/platform-tools-latest-windows.zip" \
            -o "$TMPDIR/platform-tools.zip"
        rm -rf "$TMPDIR/pt-extract"
        unzip -q -o "$TMPDIR/platform-tools.zip" -d "$TMPDIR/pt-extract"
        cp "$TMPDIR/pt-extract/platform-tools/adb.exe"          "$RESOURCES/bin/adb.exe"
        cp "$TMPDIR/pt-extract/platform-tools/AdbWinApi.dll"    "$RESOURCES/bin/AdbWinApi.dll"
        cp "$TMPDIR/pt-extract/platform-tools/AdbWinUsbApi.dll" "$RESOURCES/bin/AdbWinUsbApi.dll"
        echo "  -> adb.exe + DLLs copied"
    fi

    # --- apktool.jar ---
    echo "[3/5] apktool..."
    if [ -f "$RESOURCES/apktool.jar" ]; then
        echo "  -> Already present"
    else
        # Official apktool releases (iBotPeaches).
        if curl -fL "https://github.com/iBotPeaches/Apktool/releases/download/v${APKTOOL_VERSION}/apktool_${APKTOOL_VERSION}.jar" \
            -o "$RESOURCES/apktool.jar"; then
            echo "  -> Downloaded apktool v${APKTOOL_VERSION} (iBotPeaches)"
        else
            rm -f "$RESOURCES/apktool.jar"
            echo "  !! Download failed. Place apktool.jar manually in $RESOURCES/apktool.jar"
            echo "     Direct URL: https://github.com/iBotPeaches/Apktool/releases/tag/v${APKTOOL_VERSION}"
        fi
    fi

    # --- apksigner + zipalign ---
    # Resolved from ANDROID_HOME or common Android Studio locations.
    echo "[4/5] apksigner + zipalign..."
    local SDK=""
    if [ -n "${ANDROID_HOME:-}" ] && [ -d "$ANDROID_HOME/build-tools" ]; then
        SDK="$ANDROID_HOME"
    elif [ -n "${ANDROID_SDK_ROOT:-}" ] && [ -d "$ANDROID_SDK_ROOT/build-tools" ]; then
        SDK="$ANDROID_SDK_ROOT"
    elif [ -d "${LOCALAPPDATA:-$HOME/AppData/Local}/Android/Sdk/build-tools" ]; then
        SDK="${LOCALAPPDATA:-$HOME/AppData/Local}/Android/Sdk"
    fi

    if [ -n "$SDK" ]; then
        LATEST=$(ls -1 "$SDK/build-tools" | sort -V | tail -1)
        BT="$SDK/build-tools/$LATEST"
        if [ -f "$BT/lib/apksigner.jar" ]; then
            cp "$BT/lib/apksigner.jar" "$RESOURCES/apksigner.jar"
            echo "  -> apksigner.jar copied from $BT/lib"
        fi
        if [ -f "$BT/zipalign.exe" ]; then
            cp "$BT/zipalign.exe" "$RESOURCES/bin/zipalign.exe"
            echo "  -> zipalign.exe copied from $BT"
        fi
    else
        echo "  !! Android SDK not found. Set ANDROID_HOME or install Android Studio."
        echo "     Expected layout: <SDK>/build-tools/<version>/{lib/apksigner.jar,zipalign.exe}"
        echo "     Fallback: jarsigner (from JDK) will be used instead for signing."
    fi
}

case "$PLATFORM" in
    macos)   collect_macos ;;
    windows) collect_windows ;;
    *)
        echo "!! Platform '$PLATFORM' is not supported by this script (only macos/windows)."
        exit 1
        ;;
esac

# ---------------------------------------------------------------------------
# JRE via jlink — cross-platform (requires a JDK in PATH).
# ---------------------------------------------------------------------------
echo "[5/5] JRE (via jlink)..."
JRE_DIR="$RESOURCES/jre"
JAVA_BIN="java"
if [ "$PLATFORM" = "windows" ]; then
    JAVA_BIN="java.exe"
fi

if [ -d "$JRE_DIR/bin" ] && [ -f "$JRE_DIR/bin/$JAVA_BIN" ]; then
    echo "  -> Already present ($(du -sh "$JRE_DIR" 2>/dev/null | cut -f1))"
else
    JLINK=$(which jlink 2>/dev/null || true)
    if [ -n "$JLINK" ]; then
        MODULES="java.base,java.desktop,java.logging,jdk.crypto.ec,java.security.jgss,java.naming"
        rm -rf "$JRE_DIR"
        "$JLINK" \
            --add-modules "$MODULES" \
            --output "$JRE_DIR" \
            --no-header-files \
            --no-man-pages \
            --strip-debug \
            --compress=zip-6
        rm -rf "$JRE_DIR/legal"
        # Replace any remaining symlinks with real copies (no-op on Windows).
        find "$JRE_DIR" -type l 2>/dev/null | while read -r link; do
            target=$(readlink -f "$link" 2>/dev/null || true)
            if [ -n "$target" ] && [ -e "$target" ]; then
                rm "$link"
                cp "$target" "$link"
            fi
        done
        echo "  -> Built minimal JRE ($(du -sh "$JRE_DIR" 2>/dev/null | cut -f1))"
    else
        echo "  !! jlink not found. Install a JDK 21 and ensure it is in PATH."
    fi
fi

echo ""
echo "=== Summary ==="
ls -lh "$RESOURCES"/ 2>/dev/null || true
echo ""
ls -lh "$RESOURCES/bin/" 2>/dev/null || true
echo ""
echo "Done! You can now run: cargo tauri build"
