use std::path::{Path, PathBuf};

use serde::Serialize;
use tokio::process::Command;

use crate::error::DokkyError;

const DOFUS_PACKAGE: &str = "com.ankama.dofustouch";

/// Metadata for a clone, read directly from the device.
#[derive(Debug, Clone, Serialize)]
pub struct CloneInfo {
    pub package: String,
    pub display_name: String,
    /// Base64-encoded PNG icon
    pub icon: Option<String>,
}

/// List all Dofus Touch packages installed on a device.
/// Reads the real app name from each APK in parallel.
pub async fn list_dofus_clones(serial: &str) -> Result<Vec<CloneInfo>, DokkyError> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "pm", "list", "packages"])
        .output()
        .await
        .map_err(|_| DokkyError::AdbNotFound)?;

    if !output.status.success() {
        return Err(DokkyError::AdbCommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            let pkg = line.trim().strip_prefix("package:")?;
            if pkg.starts_with(DOFUS_PACKAGE) {
                Some(pkg.to_string())
            } else {
                None
            }
        })
        .collect();

    // Resolve app names + icons in parallel (one pull+decompile per clone)
    let serial_owned = serial.to_string();
    let mut tasks = Vec::new();
    for pkg in packages {
        let s = serial_owned.clone();
        let p = pkg.clone();
        tasks.push(tokio::spawn(async move {
            let (name, icon) = read_app_info_from_device(&s, &p).await.unwrap_or_else(|_| {
                let fallback = if p == DOFUS_PACKAGE {
                    "Dofus Touch (original)".to_string()
                } else {
                    let suffix = p.strip_prefix(DOFUS_PACKAGE).unwrap_or("");
                    format!("Dofus Touch {}", suffix)
                };
                (fallback, None)
            });
            CloneInfo { package: p, display_name: name, icon }
        }));
    }

    let mut clones = Vec::new();
    for task in tasks {
        if let Ok(info) = task.await {
            clones.push(info);
        }
    }

    Ok(clones)
}

/// Read app name + icon from a package's APK on the device.
/// Pulls the base APK, extracts icon from zip, decompiles for name.
async fn read_app_info_from_device(
    serial: &str,
    package: &str,
) -> Result<(String, Option<String>), DokkyError> {
    let work_dir = std::env::temp_dir().join(format!("dokky_info_{}", package));
    std::fs::create_dir_all(&work_dir).ok();

    let result = async {
        let paths = get_all_apk_paths(serial, package).await?;
        let base = paths.iter().find(|p| p.contains("base")).or(paths.first())
            .ok_or_else(|| DokkyError::ApkCloneFailed("no APK".to_string()))?;

        let local = work_dir.join("base.apk");
        run_adb(serial, &["pull", base, local.to_str().unwrap()]).await?;

        // Extract icon directly from APK zip (fast, no decompile needed)
        let icon_b64 = extract_icon_from_apk(&local, &work_dir)
            .and_then(|path| {
                let bytes = std::fs::read(&path)
                    .map_err(|e| DokkyError::ApkCloneFailed(e.to_string()))?;
                use base64::Engine;
                Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
            })
            .ok();

        // Decompile for app name
        let dec = work_dir.join("dec");
        run_cmd("apktool", &[
            "d", local.to_str().unwrap(),
            "-o", dec.to_str().unwrap(),
            "-f", "--no-src",
        ])
        .await
        .map_err(|e| DokkyError::ApkCloneFailed(e))?;

        let mut name = package.to_string();
        let strings_path = dec.join("res/values/strings.xml");
        if let Ok(content) = std::fs::read_to_string(&strings_path) {
            for line in content.lines() {
                if line.contains("name=\"app_name\"") {
                    if let Some(start) = line.find('>') {
                        if let Some(end) = line[start + 1..].find('<') {
                            name = line[start + 1..start + 1 + end].to_string();
                        }
                    }
                }
            }
        }

        Ok((name, icon_b64))
    }
    .await;

    std::fs::remove_dir_all(&work_dir).ok();
    result
}

/// Clone the Dofus Touch APK with a new package name.
/// Approach (proven from existing clone_dofus_clean.sh):
///   1. Pull base APK + all splits
///   2. Decompile only the base with apktool, change package name, recompile
///   3. Copy splits as-is (don't modify their package name)
///   4. Zipalign + sign ALL APKs with the same key
///   5. Install with adb install-multiple
/// Clone with custom display name and optional icon tint color (hex: "#FF5555").
pub async fn clone_dofus(
    serial: &str,
    clone_suffix: &str,
    display_name: &str,
    icon_color: Option<&str>,
) -> Result<String, DokkyError> {
    check_tools().await?;

    let new_package = format!("{}{}", DOFUS_PACKAGE, clone_suffix);
    log::info!("[apk] Cloning {} -> {} (name: '{}', color: {:?})",
        DOFUS_PACKAGE, new_package, display_name, icon_color);

    // Check if already installed
    let existing = list_dofus_clones(serial).await.unwrap_or_default();
    if existing.iter().any(|c| c.package == new_package) {
        log::info!("[apk] {} already installed, skipping", new_package);
        return Ok(new_package);
    }

    let work_dir = std::env::temp_dir().join(format!("dokky_clone_{}", clone_suffix));
    if work_dir.exists() {
        std::fs::remove_dir_all(&work_dir).ok();
    }
    std::fs::create_dir_all(&work_dir)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("create work dir: {}", e)))?;

    let result = do_clone(serial, &new_package, display_name, icon_color, &work_dir).await;

    std::fs::remove_dir_all(&work_dir).ok();
    result
}

/// Extract the Dofus Touch app icon as base64 PNG for display in the UI.
pub async fn get_dofus_icon(serial: &str) -> Result<String, DokkyError> {
    let apk_paths = get_all_apk_paths(serial, DOFUS_PACKAGE).await?;
    let base_remote = apk_paths.iter()
        .find(|p| p.contains("base.apk"))
        .or(apk_paths.first())
        .ok_or_else(|| DokkyError::ApkCloneFailed("no base APK found".to_string()))?;

    let work_dir = std::env::temp_dir().join("dokky_icon");
    std::fs::create_dir_all(&work_dir).ok();
    let local_apk = work_dir.join("base.apk");

    // Pull APK if not cached
    if !local_apk.exists() {
        run_adb(serial, &["pull", base_remote, local_apk.to_str().unwrap()]).await?;
    }

    // Extract icon from APK (it's a zip)
    let icon_path = extract_icon_from_apk(&local_apk, &work_dir)?;

    // Read as base64
    let icon_bytes = std::fs::read(&icon_path)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("read icon: {}", e)))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(&icon_bytes))
}

async fn do_clone(
    serial: &str,
    new_package: &str,
    display_name: &str,
    icon_color: Option<&str>,
    work_dir: &Path,
) -> Result<String, DokkyError> {
    // 1. Get base APK path
    log::info!("[apk] Finding base APK on device...");
    let apk_paths = get_all_apk_paths(serial, DOFUS_PACKAGE).await?;
    let base_remote = apk_paths.iter()
        .find(|p| p.contains("base.apk"))
        .or(apk_paths.first())
        .ok_or_else(|| DokkyError::ApkCloneFailed("no base APK found".to_string()))?
        .clone();

    // 2. Pull only the base APK (splits won't be needed — we remove the split requirement)
    let base_apk = work_dir.join("base.apk");
    log::info!("[apk] Pulling base APK...");
    run_adb(serial, &["pull", &base_remote, base_apk.to_str().unwrap()]).await?;

    // 3. Decompile BASE APK only, modify package name, recompile
    let decompiled_dir = work_dir.join("decompiled");
    log::info!("[apk] Decompiling base APK...");
    run_cmd("apktool", &[
        "d", base_apk.to_str().unwrap(),
        "-o", decompiled_dir.to_str().unwrap(),
        "-f",
    ])
    .await
    .map_err(|e| DokkyError::ApkCloneFailed(format!("apktool decompile: {}", e)))?;

    // 4. Modify package name in manifest + resources
    log::info!("[apk] Modifying package name to {}...", new_package);
    let manifest_path = decompiled_dir.join("AndroidManifest.xml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("read manifest: {}", e)))?;

    // Only remove split attributes from manifest (DON'T replace package name here —
    // replace_in_files_recursive will handle ALL files including manifest in one pass)
    let mut modified = manifest.clone();

    // Remove split requirements so the base APK installs standalone
    for attr_prefix in &["android:requiredSplitTypes=\"", "android:splitTypes=\""] {
        while let Some(start) = modified.find(attr_prefix) {
            let after_prefix = start + attr_prefix.len();
            if let Some(close_quote) = modified[after_prefix..].find('"') {
                let mut end = after_prefix + close_quote + 1;
                while end < modified.len() && modified.as_bytes()[end] == b' ' {
                    end += 1;
                }
                modified = format!("{}{}", &modified[..start], &modified[end..]);
            } else {
                break;
            }
        }
    }
    modified = modified.replace("android:extractNativeLibs=\"false\"", "android:extractNativeLibs=\"true\"");

    std::fs::write(&manifest_path, &modified)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("write manifest: {}", e)))?;

    // Rename smali directories BEFORE doing text replacements
    let smali_dirs = ["smali", "smali_classes2", "smali_classes3"];
    let old_path = DOFUS_PACKAGE.replace('.', "/");
    let new_path = new_package.replace('.', "/");
    for smali_dir_name in &smali_dirs {
        let old_dir = decompiled_dir.join(smali_dir_name).join(&old_path);
        let new_dir = decompiled_dir.join(smali_dir_name).join(&new_path);
        if old_dir.exists() {
            if let Some(parent) = new_dir.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::rename(&old_dir, &new_dir).ok();
            log::info!("[apk] Renamed smali dir: {} -> {}", smali_dir_name, new_path);
        }
    }

    // Replace ALL package references in one pass across all files (manifest, smali, xml, yml)
    log::info!("[apk] Replacing package references in all files...");
    replace_in_files_recursive(
        &decompiled_dir,
        &["smali", "xml", "yml"],
        DOFUS_PACKAGE,
        new_package,
    );

    // Update app name in strings.xml with custom display name
    let app_label = if display_name.is_empty() { new_package.to_string() } else { display_name.to_string() };
    log::info!("[apk] Setting app name to '{}'", app_label);
    let strings_path = decompiled_dir.join("res/values/strings.xml");
    if strings_path.exists() {
        if let Ok(strings) = std::fs::read_to_string(&strings_path) {
            let modified_strings = strings.replace(
                ">DOFUS Touch<",
                &format!(">{}<", app_label),
            );
            std::fs::write(&strings_path, modified_strings).ok();
        }
    }
    // Also update all localized strings
    if let Ok(entries) = std::fs::read_dir(decompiled_dir.join("res")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("values-") {
                let lang_strings = entry.path().join("strings.xml");
                if lang_strings.exists() {
                    if let Ok(content) = std::fs::read_to_string(&lang_strings) {
                        std::fs::write(&lang_strings, content.replace(">DOFUS Touch<", &format!(">{}<", app_label))).ok();
                    }
                }
            }
        }
    }

    // Tint the app icon if a color was provided
    if let Some(color_hex) = icon_color {
        log::info!("[apk] Tinting icon with color {}", color_hex);
        tint_icons(&decompiled_dir, color_hex);
    }

    // 5. Recompile base APK
    let rebuilt_base = work_dir.join("rebuilt_base.apk");
    log::info!("[apk] Recompiling base APK...");
    run_cmd("apktool", &[
        "b", decompiled_dir.to_str().unwrap(),
        "-o", rebuilt_base.to_str().unwrap(),
    ])
    .await
    .map_err(|e| DokkyError::ApkCloneFailed(format!("apktool recompile: {}", e)))?;

    // 6. Zipalign + sign base APK
    let keystore = get_or_create_keystore()?;
    let aligned_base = work_dir.join("aligned_base.apk");
    log::info!("[apk] Aligning and signing...");
    zipalign(&rebuilt_base, &aligned_base).await?;
    sign_apk(&aligned_base, &keystore).await?;

    // 7. Install standalone base APK (no splits needed — requiredSplitTypes removed)
    log::info!("[apk] Installing {}...", new_package);
    match run_adb(serial, &["install", "-r", aligned_base.to_str().unwrap()]).await {
        Ok(output) => log::info!("[apk] Install output: {}", output.trim()),
        Err(e) => {
            log::error!("[apk] Install failed: {}", e);
            return Err(e);
        }
    }

    log::info!("[apk] Clone '{}' installed successfully!", new_package);
    Ok(new_package.to_string())
}

/// Remove a cloned Dofus package from the device.
pub async fn remove_clone(serial: &str, package: &str) -> Result<(), DokkyError> {
    if package == DOFUS_PACKAGE {
        return Err(DokkyError::ApkCloneFailed(
            "cannot remove the original Dofus Touch package".to_string(),
        ));
    }
    if !package.starts_with(DOFUS_PACKAGE) {
        return Err(DokkyError::ApkCloneFailed(
            "can only remove Dofus Touch clones".to_string(),
        ));
    }

    log::info!("[apk] Uninstalling {}...", package);
    run_adb(serial, &["uninstall", package]).await?;

    log::info!("[apk] {} removed", package);
    Ok(())
}

// --- Internal helpers ---

/// Recursively replace a string in all files matching given extensions.
fn replace_in_files_recursive(dir: &Path, extensions: &[&str], old: &str, new: &str) {
    let old_slash = old.replace('.', "/");
    let new_slash = new.replace('.', "/");

    let walker = walkdir(dir);
    for entry in walker {
        if let Some(ext) = entry.extension().and_then(|e| e.to_str()) {
            if extensions.iter().any(|&e| e == ext) {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    let modified = content
                        .replace(old, new)
                        .replace(&old_slash, &new_slash);
                    if modified != content {
                        std::fs::write(&entry, modified).ok();
                    }
                }
            }
        }
    }
}

/// Simple recursive directory walker returning file paths.
fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

async fn check_tools() -> Result<(), DokkyError> {
    for tool in ["apktool", "java"] {
        let check = Command::new("which").arg(tool).output().await;
        match check {
            Ok(out) if out.status.success() => {}
            _ => return Err(DokkyError::ToolNotFound(tool.to_string())),
        }
    }

    let has_apksigner = Command::new("which").arg("apksigner").output().await
        .map(|o| o.status.success()).unwrap_or(false);
    let has_jarsigner = Command::new("which").arg("jarsigner").output().await
        .map(|o| o.status.success()).unwrap_or(false);

    if !has_apksigner && !has_jarsigner {
        return Err(DokkyError::ToolNotFound("apksigner or jarsigner".to_string()));
    }

    Ok(())
}

async fn get_all_apk_paths(serial: &str, package: &str) -> Result<Vec<String>, DokkyError> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "pm", "path", package])
        .output()
        .await
        .map_err(|_| DokkyError::AdbNotFound)?;

    if !output.status.success() {
        return Err(DokkyError::ApkCloneFailed(format!(
            "package '{}' not found on device", package
        )));
    }

    let paths: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("package:").map(String::from))
        .collect();

    if paths.is_empty() {
        return Err(DokkyError::ApkCloneFailed(format!(
            "no APK files found for {}", package
        )));
    }

    Ok(paths)
}

async fn run_adb(serial: &str, args: &[&str]) -> Result<String, DokkyError> {
    let mut cmd_args = vec!["-s", serial];
    cmd_args.extend_from_slice(args);

    let output = Command::new("adb")
        .args(&cmd_args)
        .output()
        .await
        .map_err(|_| DokkyError::AdbNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(DokkyError::AdbCommandFailed(stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("{}: {}", program, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("{}\n{}", stderr, stdout));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn get_or_create_keystore() -> Result<PathBuf, DokkyError> {
    let dokky_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dokky");
    std::fs::create_dir_all(&dokky_dir).ok();

    let keystore = dokky_dir.join("debug.keystore");
    if !keystore.exists() {
        log::info!("[apk] Generating debug keystore...");
        let status = std::process::Command::new("keytool")
            .args([
                "-genkey", "-v",
                "-keystore", keystore.to_str().unwrap(),
                "-alias", "dokky",
                "-keyalg", "RSA",
                "-keysize", "2048",
                "-validity", "10000",
                "-storepass", "dokky123",
                "-keypass", "dokky123",
                "-dname", "CN=Dokky,OU=Dokky,O=Dokky,L=Unknown,ST=Unknown,C=XX",
            ])
            .output()
            .map_err(|e| DokkyError::ApkCloneFailed(format!("keytool: {}", e)))?;

        if !status.status.success() {
            return Err(DokkyError::ApkCloneFailed("failed to generate debug keystore".to_string()));
        }
    }

    Ok(keystore)
}

/// Extract the highest-res launcher icon from an APK (zip).
fn extract_icon_from_apk(apk_path: &Path, output_dir: &Path) -> Result<PathBuf, DokkyError> {
    // APK is a zip — look for mipmap ic_launcher.png (prefer highest density)
    let file = std::fs::File::open(apk_path)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("open apk: {}", e)))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("read zip: {}", e)))?;

    // Search all possible icon paths (varies between original and recompiled APKs)
    let mut icon_candidates: Vec<String> = Vec::new();
    let densities = ["xxxhdpi", "xxhdpi", "xhdpi", "hdpi", "mdpi"];
    for d in &densities {
        for suffix in &["-v4", "-v26", ""] {
            icon_candidates.push(format!("res/mipmap-{}{}/ic_launcher.png", d, suffix));
        }
    }

    for candidate in icon_candidates.iter() {
        if let Ok(mut entry) = archive.by_name(candidate) {
            let output_path = output_dir.join("ic_launcher.png");
            let mut output_file = std::fs::File::create(&output_path)
                .map_err(|e| DokkyError::ApkCloneFailed(format!("create icon file: {}", e)))?;
            std::io::copy(&mut entry, &mut output_file)
                .map_err(|e| DokkyError::ApkCloneFailed(format!("extract icon: {}", e)))?;
            return Ok(output_path);
        }
    }

    Err(DokkyError::ApkCloneFailed("no launcher icon found in APK".to_string()))
}

/// Parse a hex color string like "#FF5555" into (r, g, b).
fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Tint all launcher icons in a decompiled APK directory with a color overlay.
fn tint_icons(decompiled_dir: &Path, color_hex: &str) {
    let (tr, tg, tb) = match parse_hex_color(color_hex) {
        Some(c) => c,
        None => {
            log::warn!("[apk] Invalid color '{}', skipping tint", color_hex);
            return;
        }
    };

    let res_dir = decompiled_dir.join("res");
    if !res_dir.exists() {
        return;
    }

    // Find all ic_launcher PNG files
    for entry in walkdir(&res_dir) {
        let name = entry.file_name().unwrap_or_default().to_string_lossy().to_string();
        if name == "ic_launcher.png" || name == "ic_launcher_foreground.png" {
            if let Ok(img) = image::open(&entry) {
                let mut rgba = img.to_rgba8();
                // Apply color multiply blend: pixel = pixel * tint_color / 255
                for pixel in rgba.pixels_mut() {
                    let [r, g, b, a] = pixel.0;
                    if a == 0 {
                        continue; // skip transparent
                    }
                    // Strong color tint: 80% tint color, 20% original
                    pixel.0[0] = ((r as u16 * 20 + tr as u16 * 80) / 100).min(255) as u8;
                    pixel.0[1] = ((g as u16 * 20 + tg as u16 * 80) / 100).min(255) as u8;
                    pixel.0[2] = ((b as u16 * 20 + tb as u16 * 80) / 100).min(255) as u8;
                }
                if let Err(e) = rgba.save(&entry) {
                    log::warn!("[apk] Failed to save tinted icon {:?}: {}", entry, e);
                } else {
                    log::info!("[apk] Tinted {:?}", entry.file_name().unwrap_or_default());
                }
            }
        }
    }
}

/// Zipalign an APK (required before signing with apksigner).
async fn zipalign(input: &Path, output: &Path) -> Result<(), DokkyError> {
    // Try zipalign if available, otherwise skip (apksigner works without it, just less optimal)
    let result = Command::new("zipalign")
        .args(["-p", "-f", "4", input.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .await;

    match result {
        Ok(out) if out.status.success() => Ok(()),
        _ => {
            // zipalign not available, just copy
            log::warn!("[apk] zipalign not found, skipping alignment");
            std::fs::copy(input, output)
                .map_err(|e| DokkyError::ApkCloneFailed(format!("copy: {}", e)))?;
            Ok(())
        }
    }
}

/// Sign an APK in-place using apksigner or jarsigner.
async fn sign_apk(apk: &Path, keystore: &Path) -> Result<(), DokkyError> {
    // Try apksigner first
    let result = Command::new("apksigner")
        .args([
            "sign",
            "--ks", keystore.to_str().unwrap(),
            "--ks-pass", "pass:dokky123",
            "--key-pass", "pass:dokky123",
            "--ks-key-alias", "dokky",
            apk.to_str().unwrap(),
        ])
        .output()
        .await;

    match result {
        Ok(out) if out.status.success() => return Ok(()),
        Ok(out) => log::warn!("[apk] apksigner failed: {}, trying jarsigner...",
            String::from_utf8_lossy(&out.stderr)),
        Err(_) => log::warn!("[apk] apksigner not available, trying jarsigner..."),
    }

    // Fallback: jarsigner
    let jarsigner = Command::new("jarsigner")
        .args([
            "-verbose", "-sigalg", "SHA256withRSA", "-digestalg", "SHA-256",
            "-keystore", keystore.to_str().unwrap(),
            "-storepass", "dokky123",
            "-keypass", "dokky123",
            apk.to_str().unwrap(),
            "dokky",
        ])
        .output()
        .await
        .map_err(|e| DokkyError::ApkCloneFailed(format!("jarsigner: {}", e)))?;

    if !jarsigner.status.success() {
        return Err(DokkyError::ApkCloneFailed(format!(
            "signing failed: {}", String::from_utf8_lossy(&jarsigner.stderr)
        )));
    }

    Ok(())
}
