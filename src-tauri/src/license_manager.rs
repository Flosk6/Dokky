use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::DokkyError;

const LEMON_SQUEEZY_API: &str = "https://api.lemonsqueezy.com/v1/licenses";

/// Stored license data in ~/.dokky/license.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredLicense {
    pub license_key: String,
    pub instance_id: String,
}

/// License status returned to the frontend
#[derive(Debug, Clone, Serialize)]
pub struct LicenseStatus {
    pub is_pro: bool,
    pub license_key: Option<String>,
    /// Masked key for display (e.g. "DOKKY-XXXX-****-****")
    pub display_key: Option<String>,
    pub error: Option<String>,
}

fn license_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dokky")
        .join("license.json")
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    let visible = &key[..8];
    format!("{}****", visible)
}

/// Get a unique instance ID for this machine (generated once, stored alongside license).
fn get_or_create_instance_name() -> String {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dokky")
        .join("instance_id");

    if let Ok(id) = std::fs::read_to_string(&path) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }

    let id = format!("dokky-{}", uuid::Uuid::new_v4());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, &id).ok();
    id
}

/// Load stored license from disk.
pub fn load_license() -> Option<StoredLicense> {
    let path = license_path();
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save license to disk.
fn save_license(license: &StoredLicense) -> Result<(), DokkyError> {
    let path = license_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| DokkyError::ApkCloneFailed(format!("create license dir: {}", e)))?;
    }
    let data = serde_json::to_string_pretty(license)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("serialize license: {}", e)))?;
    std::fs::write(&path, data)
        .map_err(|e| DokkyError::ApkCloneFailed(format!("write license: {}", e)))?;
    Ok(())
}

/// Delete stored license.
fn delete_license() {
    let path = license_path();
    std::fs::remove_file(&path).ok();
}

/// Validate a stored license with LemonSqueezy.
pub async fn validate_license() -> LicenseStatus {
    let stored = match load_license() {
        Some(l) => l,
        None => return LicenseStatus {
            is_pro: false,
            license_key: None,
            display_key: None,
            error: None,
        },
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/validate", LEMON_SQUEEZY_API))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "license_key": stored.license_key,
            "instance_id": stored.instance_id,
        }))
        .send()
        .await;

    match resp {
        Ok(r) => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let valid = body["valid"].as_bool().unwrap_or(false);
            if valid {
                LicenseStatus {
                    is_pro: true,
                    license_key: Some(stored.license_key.clone()),
                    display_key: Some(mask_key(&stored.license_key)),
                    error: None,
                }
            } else {
                let error = body["error"].as_str().unwrap_or("Licence invalide ou expirée").to_string();
                delete_license();
                LicenseStatus {
                    is_pro: false,
                    license_key: None,
                    display_key: None,
                    error: Some(error),
                }
            }
        }
        Err(e) => {
            log::warn!("[license] Validation failed: {}", e);
            // Network error — deny access (online required)
            LicenseStatus {
                is_pro: false,
                license_key: Some(stored.license_key.clone()),
                display_key: Some(mask_key(&stored.license_key)),
                error: Some("Impossible de vérifier la licence (pas de connexion)".to_string()),
            }
        }
    }
}

/// Activate a license key on this machine.
pub async fn activate_license(license_key: &str) -> LicenseStatus {
    let instance_name = get_or_create_instance_name();

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/activate", LEMON_SQUEEZY_API))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "license_key": license_key,
            "instance_name": instance_name,
        }))
        .send()
        .await;

    match resp {
        Ok(r) => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let valid = body["activated"].as_bool().unwrap_or(false);
            if valid {
                let instance_id = body["instance"]["id"].as_str().unwrap_or("").to_string();
                let stored = StoredLicense {
                    license_key: license_key.to_string(),
                    instance_id,
                };
                if let Err(e) = save_license(&stored) {
                    log::error!("[license] Failed to save: {}", e);
                }
                LicenseStatus {
                    is_pro: true,
                    license_key: Some(license_key.to_string()),
                    display_key: Some(mask_key(license_key)),
                    error: None,
                }
            } else {
                let error = body["error"].as_str()
                    .unwrap_or("Clé invalide ou limite d'appareils atteinte")
                    .to_string();
                LicenseStatus {
                    is_pro: false,
                    license_key: None,
                    display_key: None,
                    error: Some(error),
                }
            }
        }
        Err(e) => {
            LicenseStatus {
                is_pro: false,
                license_key: None,
                display_key: None,
                error: Some(format!("Erreur de connexion: {}", e)),
            }
        }
    }
}

/// Deactivate the license on this machine (frees the slot).
pub async fn deactivate_license() -> LicenseStatus {
    let stored = match load_license() {
        Some(l) => l,
        None => return LicenseStatus {
            is_pro: false,
            license_key: None,
            display_key: None,
            error: None,
        },
    };

    let client = reqwest::Client::new();
    let _ = client
        .post(&format!("{}/deactivate", LEMON_SQUEEZY_API))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "license_key": stored.license_key,
            "instance_id": stored.instance_id,
        }))
        .send()
        .await;

    delete_license();

    LicenseStatus {
        is_pro: false,
        license_key: None,
        display_key: None,
        error: None,
    }
}
