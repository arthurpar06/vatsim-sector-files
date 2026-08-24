//! Persistence layer for the user profile. Data types live in
//! `controller_pack_core::profile_types` and the pack-detection helpers in
//! `controller_pack_core::pack_dir`; this module wraps them with
//! `tauri-plugin-store` load/save.

pub use controller_pack_core::pack_dir::{detect_pack_dir, looks_like_controller_pack};
pub use controller_pack_core::profile_types::{
    InstalledVersions, Preferences, Profile, VatsimCredentials,
};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

pub const STORE_FILE: &str = "profile.json";
pub const PROFILE_KEY: &str = "profile";

pub fn load<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<Profile> {
    let store = app.store(STORE_FILE)?;
    Ok(match store.get(PROFILE_KEY) {
        Some(value) => serde_json::from_value(value).unwrap_or_default(),
        None => Profile::default(),
    })
}

pub fn save<R: Runtime>(app: &AppHandle<R>, profile: &Profile) -> anyhow::Result<()> {
    let store = app.store(STORE_FILE)?;
    store.set(PROFILE_KEY, serde_json::to_value(profile)?);
    store.save()?;
    Ok(())
}

