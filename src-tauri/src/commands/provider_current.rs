use std::time::Duration;
use tauri::AppHandle;

const ZED_FAST_LOCAL_READ_TIMEOUT: Duration = Duration::from_secs(20);

fn resolve_provider_current_account_id(platform: &str) -> Result<Option<String>, String> {
    match platform {
        "windsurf" => {
            let accounts = crate::modules::windsurf_account::list_accounts();
            Ok(crate::modules::windsurf_account::resolve_current_account_id(&accounts))
        }
        "kiro" => {
            let accounts = crate::modules::kiro_account::list_accounts();
            Ok(crate::modules::kiro_account::resolve_current_account_id(
                &accounts,
            ))
        }
        "cursor" => {
            let accounts = crate::modules::cursor_account::list_accounts();
            Ok(crate::modules::cursor_account::resolve_current_account_id(
                &accounts,
            ))
        }
        "gemini" => {
            let accounts = crate::modules::gemini_account::list_accounts();
            Ok(
                crate::modules::gemini_account::resolve_current_account(&accounts)
                    .map(|account| account.id),
            )
        }
        "codebuddy" => {
            let accounts = crate::modules::codebuddy_account::list_accounts();
            Ok(crate::modules::codebuddy_account::resolve_current_account_id(&accounts))
        }
        "codebuddy_cn" | "codebuddy-cn" => {
            let accounts = crate::modules::codebuddy_cn_account::list_accounts();
            Ok(crate::modules::codebuddy_cn_account::resolve_current_account_id(&accounts))
        }
        "qoder" => {
            let accounts = crate::modules::qoder_account::list_accounts();
            Ok(crate::modules::qoder_account::resolve_current_account_id(
                &accounts,
            ))
        }
        "trae" => {
            let accounts = crate::modules::trae_account::list_accounts();
            Ok(crate::modules::trae_account::resolve_current_account_id(
                &accounts,
            ))
        }
        "workbuddy" => {
            let accounts = crate::modules::workbuddy_account::list_accounts();
            Ok(crate::modules::workbuddy_account::resolve_current_account_id(&accounts))
        }
        "github_copilot" | "github-copilot" | "ghcp" => {
            let accounts = crate::modules::github_copilot_account::list_accounts();
            Ok(crate::modules::github_copilot_account::resolve_current_account_id(&accounts))
        }
        "zed" => crate::modules::platform_adapter::call_zed_with_timeout(
            "accounts.current",
            serde_json::json!({}),
            ZED_FAST_LOCAL_READ_TIMEOUT,
        ),
        other => Err(format!("不支持的平台: {}", other)),
    }
}

#[tauri::command]
pub async fn get_provider_current_account_id(
    app: AppHandle,
    platform: String,
) -> Result<Option<String>, String> {
    let current_account_id = resolve_provider_current_account_id(platform.trim())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _ = crate::modules::tray::update_tray_menu(&app);
    });
    Ok(current_account_id)
}
