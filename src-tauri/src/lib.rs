use std::{
    fs::File,
    io::{self, Read, Write},
};

use serde::Serialize;
use tauri::Emitter;
use yuralock::crypto::{AEStream, BUFFER_SIZE};
use yuralock::EncryFile;

#[cfg(target_os = "android")]
mod android;

#[cfg(not(target_os = "android"))]
mod desktop;

const DEFAULT_ENCRYPT_PART: u64 = 50;
pub(crate) const FAKE_HEADER_BYTES: u64 = 8;
pub(crate) const CHECK_BYTES: u64 = 31;

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

#[derive(Clone, Serialize)]
pub(crate) struct CryptoProgressPayload {
    percent: u8,
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

pub(crate) fn emit_frontend_progress<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    percent: u8,
) -> Result<(), anyhow::Error> {
    app.emit(
        "frontend://crypto-progress",
        CryptoProgressPayload {
            percent: percent.min(100),
        },
    ).map_err(|x| x.into())
}

pub(crate) fn copy_with_progress(
    source: &mut impl Read,
    dest: &mut impl Write,
) -> io::Result<u64> {
    let mut copied = 0u64;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            return Ok(copied);
        }
        dest.write_all(&buffer[..read])?;
        copied += read as u64;
    }
}

pub(crate) fn compatible_encrypt(
    mut source: File,
    dest: File,
    source_name: String,
    source_size: u64,
    part: u64,
    key: String,
    mut on_progress: impl FnMut(u64),
) -> Result<(), anyhow::Error> {
    let mut dest = EncryFile::from_file(dest, key.clone());
    let encrypt_part_size = dest.write_header_down(source_name, source_size, part)?;

    {
        let encrypt_source = Read::by_ref(&mut source).take(encrypt_part_size);
        let mut stream = AEStream::new(encrypt_source, &mut dest)?;
        stream.set_encryptor(&key)?;
        loop {
            let next = stream.next()?;
            if next == usize::MAX {
                on_progress(BUFFER_SIZE as u64);
                continue;
            }
            on_progress(next as u64);
            stream.finalize(next)?;
            break;
        }
    }
    
    copy_with_progress(&mut source, &mut dest)?;
    dest.finilaize()?;
    Ok(())
}

fn normalize_encrypt_part(encrypt_part: Option<u64>) -> u64 {
    encrypt_part.unwrap_or(DEFAULT_ENCRYPT_PART).min(100)
}

fn validate_request(input_path: &str, key: &str) -> Result<(), String> {
    if input_path.trim().is_empty() {
        return Err("请选择文件".to_string());
    }
    if key.is_empty() {
        return Err("请输入密钥".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn peek_file_from_path(_app: tauri::AppHandle, input_path: String) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    return Ok(android::peek_file_from_uri(&_app, &input_path).await);

    #[cfg(not(target_os = "android"))]
    Ok(yuralock::pubapi::peek_file(input_path).unwrap_or(false))
}

#[tauri::command]
async fn process_file_from_path(
    _app: tauri::AppHandle,
    input_path: String,
    isencry: bool,
    key: String,
    encrypt_part: Option<u64>,
) -> Result<CryptoResult, String> {
    validate_request(&input_path, &key)?;
    let encrypt_part = normalize_encrypt_part(encrypt_part);
    let _ = emit_frontend_progress(&_app, 0);

    #[cfg(target_os = "android")]
    let result =
        android::process_file_from_android_uri(&_app, &input_path, isencry, key, encrypt_part)
            .await;

    #[cfg(not(target_os = "android"))]
    let result =
        desktop::process_file_from_path_inner(&_app, input_path, isencry, key, encrypt_part).await;

    if result.is_ok() {
        let _ = emit_frontend_progress(&_app, 100);
    }
    result
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
