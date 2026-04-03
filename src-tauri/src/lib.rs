use std::{
    fs::File,
    io::{self, Read},
};

use serde::Serialize;
use tauri::Emitter;
use yuralock::EncryFile;
use yuralock::{crypto::encrypt};

#[cfg(target_os = "android")]
mod android;

#[cfg(not(target_os = "android"))]
mod desktop;
#[cfg(not(target_os = "android"))]
use yuralock::pubapi::peek_file;
#[cfg(not(target_os = "android"))]
use std::path::Path;

const DEFAULT_ENCRYPT_PART: u64 = 20;

#[derive(Serialize)]
struct CryptoResult {
    output_path: String,
    message: String,
}

#[derive(Clone, Serialize)]
struct ToastPayload {
    message: String,
    #[serde(rename = "type")]
    toast_type: String,
}

#[tauri::command]
fn show_toast_from_backend(
    app: tauri::AppHandle,
    message: String,
    toast_type: Option<String>,
) -> Result<(), String> {
    let toast_type = toast_type.unwrap_or_else(|| "success".to_string());
    app.emit(
        "frontend://show-toast",
        ToastPayload {
            message,
            toast_type,
        },
    )
    .map_err(|e| e.to_string())
}

fn compatible_encrypt(
    mut source: File,
    dest: File,
    source_name: String,
    part: u64,
    key: String,
) -> Result<(), anyhow::Error> {
    let mut dest = EncryFile::from_file(dest, key.clone());
    let source_size = source.metadata()?.len();

    let encrypt_part_size = dest.write_header_down(source_name, source_size, part)?;
    encrypt(
        &mut source.by_ref().take(encrypt_part_size),
        &mut dest,
        &key,
    )?;
    io::copy(&mut source, &mut dest)?;
    dest.finilaize()?;
    Ok(())
}

#[tauri::command]
async fn peek_file_from_path(_app: tauri::AppHandle, input_path: String) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    return Ok(android::peek_file_from_uri(&_app, &input_path).await);

    #[cfg(not(target_os = "android"))]
    Ok(peek_file(Path::new(&input_path)).unwrap_or(false))
}

#[tauri::command]
async fn process_file_from_path(
    _app: tauri::AppHandle,
    input_path: String,
    isencry: bool,
    key: String,
    encrypt_part: Option<u64>,
) -> Result<CryptoResult, String> {
    let encrypt_part = encrypt_part.unwrap_or(DEFAULT_ENCRYPT_PART);

    #[cfg(target_os = "android")]
    return android::process_file_from_android_uri(&_app, &input_path, isencry, key, encrypt_part)
        .await;

    #[cfg(not(target_os = "android"))]
    desktop::process_file_from_path_inner(input_path, isencry, key, encrypt_part).await
}

#[tauri::command]
async fn pick_input_file(_app: tauri::AppHandle) -> String {
    #[cfg(target_os = "android")]
    return android::pick_input_file(&_app).await;

    #[cfg(not(target_os = "android"))]
    desktop::pick_input_file()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_android_fs::init());

    builder
        .invoke_handler(tauri::generate_handler![
            show_toast_from_backend,
            peek_file_from_path,
            process_file_from_path,
            pick_input_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
