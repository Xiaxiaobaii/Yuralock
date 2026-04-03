use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    time::Instant,
};

#[cfg(not(target_os = "android"))]
use rfd::FileDialog;
use serde::Serialize;
use tauri::Emitter;
use uuid::Uuid;
use yuralock::{
    crypto::{decrypt, encrypt, BlakeRead},
    pubapi::{filter_fake_header, peek_file},
    EncryFile, EncryHeader,
};

#[cfg(target_os = "android")]
mod android;

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

fn emit_frontend_toast(
    app: &tauri::AppHandle,
    message: impl Into<String>,
    toast_type: impl Into<String>,
) -> Result<(), String> {
    app.emit(
        "frontend://show-toast",
        ToastPayload {
            message: message.into(),
            toast_type: toast_type.into(),
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn show_toast_from_backend(
    app: tauri::AppHandle,
    message: String,
    toast_type: Option<String>,
) -> Result<(), String> {
    let toast_type = toast_type.unwrap_or_else(|| "success".to_string());
    emit_frontend_toast(&app, message, toast_type)
}

fn output_dir_from_input(input_path: &Path) -> Result<PathBuf, String> {
    match input_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => Ok(parent.to_path_buf()),
        _ => std::env::current_dir().map_err(|e| e.to_string()),
    }
}

fn normalize_input_path(input_path: &str) -> Result<PathBuf, String> {
    let trimmed = input_path.trim();
    if trimmed.is_empty() {
        return Err("请选择文件".to_string());
    }
    Ok(PathBuf::from(trimmed))
}

fn validate_common(input_path: &str, key: &str) -> Result<PathBuf, String> {
    let source_path = normalize_input_path(input_path)?;
    if key.is_empty() {
        return Err("请输入密钥".to_string());
    }
    Ok(source_path)
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

fn encrypt_file_from_path(
    source_path: PathBuf,
    key: String,
    encrypt_part: u64,
) -> Result<CryptoResult, String> {
    let source = File::open(&source_path).map_err(|e| e.to_string())?;

    let mut output_path = output_dir_from_input(&source_path)?;
    let file_name = Uuid::new_v4().to_string();
    output_path.push(&file_name);

    let dest = File::create(&output_path).map_err(|e| e.to_string())?;

    compatible_encrypt(
        source,
        dest,
        source_path
            .file_name()
            .map(|osname| osname.to_string_lossy().to_string())
            .ok_or(format!("转换源文件路径出错: {:?}", source_path))?,
        encrypt_part,
        key,
    )
    .map_err(|e| e.to_string())?;

    Ok(CryptoResult {
        output_path: output_path.to_string_lossy().to_string(),
        message: "加密成功".to_string(),
    })
}

fn decrypt_file_from_path(source_path: PathBuf, key: String) -> Result<CryptoResult, String> {
    let origin_size = fs::metadata(&source_path).map_err(|e| e.to_string())?.len();
    let mut source = BlakeRead::from_read(File::open(&source_path).map_err(|e| e.to_string())?);

    filter_fake_header(&mut source).map_err(|_| "伪装层读取失败".to_string())?;

    let encry_part: EncryHeader = EncryHeader::new(&mut source, &key)
        .map_err(|_| "读取文件头失败，文件损坏或密钥错误".to_string())?;

    let mut output_path = output_dir_from_input(&source_path)?;
    output_path.push(&encry_part.file_name);

    let mut limit_source = source.by_ref().take(encry_part.complate_encry_size());
    let mut dest = File::create(&output_path).map_err(|e| e.to_string())?;

    decrypt(&mut limit_source, &mut dest, &key)
        .map_err(|_| "解密失败，文件损坏或密钥错误".to_string())?;

    let mut no_encry_source = source
        .by_ref()
        .take(encry_part.complate_origin_size(origin_size));
    io::copy(&mut no_encry_source, &mut dest).map_err(|_| "原始内容拷贝失败".to_string())?;

    if !source.hashcheck(&key).unwrap_or(false) {
        return Err("文件校验失败，与原始文件不一致".to_string());
    }

    Ok(CryptoResult {
        output_path: output_path.to_string_lossy().to_string(),
        message: "解密成功".to_string(),
    })
}

#[tauri::command]
async fn peek_file_from_path(_app: tauri::AppHandle, input_path: String) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    {
        return Ok(android::peek_file_from_uri(&_app, &input_path).await);
    }

    #[cfg(not(target_os = "android"))]
    {
        Ok(peek_file(Path::new(&input_path)).unwrap_or(false))
    }
}

async fn process_file_from_path_inner(
    input_path: String,
    isencry: bool,
    key: String,
    encrypt_part: u64,
) -> Result<CryptoResult, String> {
    let source_path = validate_common(&input_path, &key)?;
    let start = Instant::now();
    let result = if isencry {
        decrypt_file_from_path(source_path, key)
    } else {
        encrypt_file_from_path(source_path, key, encrypt_part)
    };
    let duration = start.elapsed();
    println!("Processing time: {:?}", duration);
    result
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
    process_file_from_path_inner(input_path, isencry, key, encrypt_part).await
}

#[tauri::command]
async fn pick_input_file(_app: tauri::AppHandle) -> String {
    #[cfg(target_os = "android")]
    {
        match android::pick_input_file(&_app).await {
            Ok(path) => return path,
            Err(error) => {
                let _ = emit_frontend_toast(&_app, error, "error");
                return String::new();
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    FileDialog::new()
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
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
