use crate::modules::platform_package::{self, PlatformPackageState};
use tauri::AppHandle;

#[tauri::command]
pub fn list_platform_packages(app: AppHandle) -> Result<Vec<PlatformPackageState>, String> {
    platform_package::list_platform_packages(&app)
}

#[tauri::command]
pub fn install_platform_package(
    app: AppHandle,
    platform_id: String,
) -> Result<PlatformPackageState, String> {
    let state = platform_package::install_platform_package(&app, platform_id.as_str())?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(state)
}

#[tauri::command]
pub fn update_platform_package(
    app: AppHandle,
    platform_id: String,
) -> Result<PlatformPackageState, String> {
    let state = platform_package::update_platform_package(&app, platform_id.as_str())?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(state)
}

#[tauri::command]
pub fn uninstall_platform_package(
    app: AppHandle,
    platform_id: String,
) -> Result<PlatformPackageState, String> {
    let state = platform_package::uninstall_platform_package(Some(&app), platform_id.as_str())?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(state)
}
