#!/bin/bash
# Collect external dependencies into src-tauri/resources/ for bundling.
# Run this before `cargo tauri build`.
#
# Usage: ./scripts/collect-deps.sh [macos|windows]

set -euo pipefail

PLATFORM="${1:-$(uname -s | tr '[:upper:]' '[:lower:]')}"
RESOURCES="src-tauri/resources"

mkdir -p "$RESOURCES/bin"

echo "=== Collecting dependencies for platform: $PLATFORM ==="

# --- scrcpy-server ---
echo "[1/4] scrcpy-server..."
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
echo "[2/4] adb..."
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
echo "[3/4] apktool..."
# Homebrew installs apktool as a wrapper script; we need the actual JAR
APKTOOL_JAR=""
# Try exact name first, then versioned names (apktool_X.Y.Z.jar)
for dir in /opt/homebrew/opt/apktool/libexec /opt/homebrew/Cellar/apktool/*/libexec /usr/local/opt/apktool/libexec; do
    if [ -f "$dir/apktool.jar" ]; then
        APKTOOL_JAR="$dir/apktool.jar"
        break
    fi
    # Find versioned jar (e.g. apktool_3.0.1.jar)
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

# --- apksigner ---
echo "[4/4] apksigner..."
# apksigner is part of Android SDK build-tools
APKSIGNER_JAR=""
if [ -d "$HOME/Library/Android/sdk/build-tools" ]; then
    # Find the latest version
    LATEST=$(ls -1 "$HOME/Library/Android/sdk/build-tools" | sort -V | tail -1)
    if [ -f "$HOME/Library/Android/sdk/build-tools/$LATEST/lib/apksigner.jar" ]; then
        APKSIGNER_JAR="$HOME/Library/Android/sdk/build-tools/$LATEST/lib/apksigner.jar"
    fi
fi

if [ -n "$APKSIGNER_JAR" ]; then
    cp "$APKSIGNER_JAR" "$RESOURCES/apksigner.jar"
    echo "  -> Copied from $APKSIGNER_JAR"
elif [ -f "$RESOURCES/apksigner.jar" ]; then
    echo "  -> Already present"
else
    echo "  !! apksigner.jar not found. Install Android SDK build-tools."
    echo "     Fallback: jarsigner (from JDK) will be used instead."
fi

# --- zipalign (optional) ---
ZIPALIGN_PATH=""
if [ -d "$HOME/Library/Android/sdk/build-tools" ]; then
    LATEST=$(ls -1 "$HOME/Library/Android/sdk/build-tools" | sort -V | tail -1)
    ZIPALIGN_PATH="$HOME/Library/Android/sdk/build-tools/$LATEST/zipalign"
fi

if [ -n "$ZIPALIGN_PATH" ] && [ -f "$ZIPALIGN_PATH" ]; then
    cp "$ZIPALIGN_PATH" "$RESOURCES/bin/zipalign"
    chmod +x "$RESOURCES/bin/zipalign"
    echo "  -> zipalign copied from $ZIPALIGN_PATH"
fi

# --- Minimal JRE (via jlink) ---
echo "[5/5] JRE (via jlink)..."
JRE_DIR="$RESOURCES/jre"
if [ -d "$JRE_DIR/bin" ] && [ -f "$JRE_DIR/bin/java" ]; then
    echo "  -> Already present ($(du -sh "$JRE_DIR" | cut -f1))"
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
        # Remove legal dir (not needed at runtime, causes Tauri build issues with symlinks)
        rm -rf "$JRE_DIR/legal"
        # Replace any remaining symlinks with real copies
        find "$JRE_DIR" -type l | while read link; do
            target=$(readlink -f "$link")
            rm "$link"
            cp "$target" "$link"
        done
        echo "  -> Built minimal JRE ($(du -sh "$JRE_DIR" | cut -f1))"
    else
        echo "  !! jlink not found. Install a JDK to bundle a minimal JRE."
    fi
fi

echo ""
echo "=== Summary ==="
echo "Resources directory:"
ls -lh "$RESOURCES"/ 2>/dev/null || true
echo ""
ls -lh "$RESOURCES/bin/" 2>/dev/null || true
echo ""
echo "Done! You can now run: cargo tauri build"
