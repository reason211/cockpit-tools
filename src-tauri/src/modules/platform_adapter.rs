use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use crate::modules::{logger, platform_package};

const ZED_PLATFORM_ID: &str = "zed";
const ADAPTER_BOOT_TIMEOUT: Duration = Duration::from_secs(10);
const ADAPTER_CALL_TIMEOUT: Duration = Duration::from_secs(180);
const ADAPTER_STOP_TIMEOUT: Duration = Duration::from_secs(3);

static PLATFORM_ADAPTERS: std::sync::LazyLock<Mutex<HashMap<String, AdapterProcess>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
struct AdapterProcess {
    package_dir: PathBuf,
    executable_path: PathBuf,
    child: Child,
    endpoint: AdapterEndpoint,
}

#[derive(Debug, Clone)]
struct AdapterEndpoint {
    url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdapterBootstrap {
    ok: bool,
    protocol: String,
    host: String,
    port: u16,
    token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdapterRequest {
    method: String,
    payload: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdapterError {
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdapterResponse {
    ok: bool,
    data: Option<Value>,
    error: Option<AdapterError>,
}

fn hidden_command(path: &PathBuf) -> Command {
    let mut command = Command::new(path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    command
}

fn child_is_running(child: &mut Child) -> bool {
    match child.try_wait() {
        Ok(Some(_)) => false,
        Ok(None) => true,
        Err(_) => false,
    }
}

fn stop_child(child: &mut Child) {
    if child_is_running(child) {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn read_bootstrap_line(child: &mut Child) -> Result<String, String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Zed adapter stdout 未捕获".to_string())?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout)
            .read_line(&mut line)
            .map(|_| line)
            .map_err(|error| format!("读取 Zed adapter 启动信息失败: {}", error));
        let _ = sender.send(result);
    });
    receiver
        .recv_timeout(ADAPTER_BOOT_TIMEOUT)
        .map_err(|_| "Zed adapter 启动超时".to_string())?
}

fn spawn_adapter(platform_id: &str) -> Result<AdapterProcess, String> {
    let installed = platform_package::installed_platform_adapter(platform_id)?;
    if installed.adapter.protocol != "http-json-v1" {
        return Err(format!(
            "平台 adapter 协议不支持: {}",
            installed.adapter.protocol
        ));
    }

    let mut command = hidden_command(&installed.executable_path);
    command
        .current_dir(&installed.current_dir)
        .env("COCKPIT_PLATFORM_ID", platform_id)
        .env("COCKPIT_PLATFORM_PACKAGE_DIR", &installed.current_dir);

    let mut child = command.spawn().map_err(|error| {
        format!(
            "启动平台 adapter 失败: platform={}, path={}, error={}",
            platform_id,
            installed.executable_path.display(),
            error
        )
    })?;

    let bootstrap_line = match read_bootstrap_line(&mut child) {
        Ok(line) => line,
        Err(error) => {
            stop_child(&mut child);
            return Err(error);
        }
    };
    let bootstrap: AdapterBootstrap =
        serde_json::from_str(bootstrap_line.trim()).map_err(|error| {
            stop_child(&mut child);
            format!("解析平台 adapter 启动信息失败: {}", error)
        })?;
    if !bootstrap.ok || bootstrap.protocol != installed.adapter.protocol {
        stop_child(&mut child);
        return Err("平台 adapter 启动握手失败".to_string());
    }
    if bootstrap.host != "127.0.0.1" || bootstrap.token.trim().is_empty() {
        stop_child(&mut child);
        return Err("平台 adapter 启动握手地址或 token 非法".to_string());
    }

    let endpoint = AdapterEndpoint {
        url: format!("http://{}:{}/rpc", bootstrap.host, bootstrap.port),
        token: bootstrap.token,
    };
    logger::log_info(&format!(
        "[PlatformAdapter] adapter 已启动: platform={}, pid={}, endpoint={}",
        platform_id,
        child.id(),
        endpoint.url
    ));

    Ok(AdapterProcess {
        package_dir: installed.current_dir,
        executable_path: installed.executable_path,
        child,
        endpoint,
    })
}

fn adapter_endpoint(platform_id: &str) -> Result<AdapterEndpoint, String> {
    let installed = platform_package::installed_platform_adapter(platform_id)?;
    let mut adapters = PLATFORM_ADAPTERS
        .lock()
        .map_err(|_| "获取平台 adapter 锁失败".to_string())?;

    let should_restart = match adapters.get_mut(platform_id) {
        Some(process) => {
            process.package_dir != installed.current_dir
                || process.executable_path != installed.executable_path
                || !child_is_running(&mut process.child)
        }
        None => true,
    };

    if should_restart {
        if let Some(mut old) = adapters.remove(platform_id) {
            stop_child(&mut old.child);
        }
        let process = spawn_adapter(platform_id)?;
        adapters.insert(platform_id.to_string(), process);
    }

    adapters
        .get(platform_id)
        .map(|process| process.endpoint.clone())
        .ok_or_else(|| format!("平台 adapter 未启动: {}", platform_id))
}

fn post_adapter_request(
    endpoint: &AdapterEndpoint,
    method: &str,
    payload: Value,
    timeout: Duration,
) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("创建平台 adapter HTTP 客户端失败: {}", error))?;
    let response = client
        .post(&endpoint.url)
        .bearer_auth(&endpoint.token)
        .json(&AdapterRequest {
            method: method.to_string(),
            payload,
        })
        .send()
        .map_err(|error| format!("调用平台 adapter 失败: {}", error))?;
    if !response.status().is_success() {
        return Err(format!("平台 adapter 返回 HTTP {}", response.status()));
    }
    let response = response
        .json::<AdapterResponse>()
        .map_err(|error| format!("解析平台 adapter 响应失败: {}", error))?;
    if response.ok {
        return Ok(response.data.unwrap_or(Value::Null));
    }
    Err(response
        .error
        .map(|error| error.message)
        .unwrap_or_else(|| "平台 adapter 调用失败".to_string()))
}

fn call_platform_adapter_value(
    platform_id: &str,
    method: &str,
    payload: Value,
) -> Result<Value, String> {
    platform_package::ensure_platform_package_installed(platform_id)?;
    let endpoint = adapter_endpoint(platform_id)?;
    post_adapter_request(&endpoint, method, payload, ADAPTER_CALL_TIMEOUT)
}

fn call_platform_adapter_value_with_timeout(
    platform_id: &str,
    method: &str,
    payload: Value,
    timeout: Duration,
) -> Result<Value, String> {
    platform_package::ensure_platform_package_installed(platform_id)?;
    let endpoint = adapter_endpoint(platform_id)?;
    post_adapter_request(&endpoint, method, payload, timeout)
}

fn existing_adapter_endpoint(platform_id: &str) -> Option<AdapterEndpoint> {
    let mut adapters = PLATFORM_ADAPTERS.lock().ok()?;
    let process = adapters.get_mut(platform_id)?;
    if child_is_running(&mut process.child) {
        Some(process.endpoint.clone())
    } else {
        None
    }
}

pub fn call_zed_value(method: &str, payload: Value) -> Result<Value, String> {
    call_platform_adapter_value(ZED_PLATFORM_ID, method, payload)
}

pub fn call_zed_with_timeout<T: DeserializeOwned>(
    method: &str,
    payload: Value,
    timeout: Duration,
) -> Result<T, String> {
    let value =
        call_platform_adapter_value_with_timeout(ZED_PLATFORM_ID, method, payload, timeout)?;
    serde_json::from_value(value).map_err(|error| format!("解析 Zed adapter 数据失败: {}", error))
}

pub fn call_zed<T: DeserializeOwned>(method: &str, payload: Value) -> Result<T, String> {
    let value = call_zed_value(method, payload)?;
    serde_json::from_value(value).map_err(|error| format!("解析 Zed adapter 数据失败: {}", error))
}

pub fn restore_zed_runtime() {
    if !platform_package::is_platform_package_installed(ZED_PLATFORM_ID) {
        return;
    }
    if let Err(error) = call_zed_value("oauth.restorePendingListener", json!({})) {
        logger::log_warn(&format!(
            "[PlatformAdapter] 恢复 Zed OAuth adapter 状态失败: {}",
            error
        ));
    }
}

pub fn stop_platform_adapter(platform_id: &str) {
    let mut adapters = match PLATFORM_ADAPTERS.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    let Some(mut process) = adapters.remove(platform_id) else {
        return;
    };
    let _ = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .and_then(|client| {
            client
                .post(&process.endpoint.url)
                .bearer_auth(&process.endpoint.token)
                .json(&AdapterRequest {
                    method: "adapter.shutdown".to_string(),
                    payload: json!({}),
                })
                .send()
        });
    stop_child(&mut process.child);
    logger::log_info(&format!(
        "[PlatformAdapter] adapter 已停止: platform={}",
        platform_id
    ));
}

pub fn stop_zed_runtime_before_uninstall() {
    if let Some(endpoint) = existing_adapter_endpoint(ZED_PLATFORM_ID) {
        if let Err(error) = post_adapter_request(
            &endpoint,
            "oauth.cancel",
            json!({ "loginId": null }),
            ADAPTER_STOP_TIMEOUT,
        ) {
            logger::log_warn(&format!(
                "[PlatformAdapter] 卸载 Zed 前取消 OAuth 状态失败，继续卸载: {}",
                error
            ));
        }
        if let Err(error) = post_adapter_request(
            &endpoint,
            "runtime.stopDefault",
            json!({}),
            ADAPTER_STOP_TIMEOUT,
        ) {
            logger::log_warn(&format!(
                "[PlatformAdapter] 卸载 Zed 前停止运行态失败，继续卸载: {}",
                error
            ));
        }
    }
    stop_platform_adapter(ZED_PLATFORM_ID);
}
