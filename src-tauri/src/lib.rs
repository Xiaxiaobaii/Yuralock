use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf}, time::Instant,
};

use anyhow::bail;
#[cfg(not(target_os = "android"))]
use rfd::FileDialog;
use tauri_plugin_android_fs::AndroidFsExt;
use serde::Serialize;
use uuid::Uuid;
use yuralock::{
    crypto::{decrypt, encrypt, BlakeRead},
    pubapi::{filter_fake_header, peek_file},
    EncryFile, EncryHeader,
};

const DEFAULT_ENCRYPT_PART: u64 = 20;

#[derive(Serialize)]
struct CryptoResult {
    output_path: String,
    message: String,
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

fn encrypt_file_from_path(source_path: PathBuf, key: String) -> Result<CryptoResult, String> {
    let mut source = File::open(&source_path).map_err(|e| e.to_string())?;

    let mut output_path = output_dir_from_input(&source_path)?;
    let file_name = Uuid::new_v4().to_string();
    output_path.push(&file_name);

    let mut dest = EncryFile::new(output_path.clone(), key.clone()).map_err(|e| e.to_string())?;

    let encrypt_part_size = dest
        .write_header(&source_path, DEFAULT_ENCRYPT_PART)
        .map_err(|e| e.to_string())?;

    encrypt(
        &mut source
            .try_clone()
            .map_err(|e| e.to_string())?
            .take(encrypt_part_size),
        &mut dest,
        &key,
    )
    .map_err(|e| e.to_string())?;

    io::copy(&mut source, &mut dest).map_err(|e| e.to_string())?;

    dest.finilaize().map_err(|e| e.to_string())?;

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
async fn peek_file_from_path(input_path: String) -> Result<bool, String> {
    let source_path = normalize_input_path(&input_path)?;
    fs::metadata(&source_path).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || Ok(peek_file(&source_path).unwrap_or(false)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn process_file_from_path(input_path: String, key: String) -> Result<CryptoResult, String> {
    let source_path = validate_common(&input_path, &key)?;
    fs::metadata(&source_path).map_err(|e| e.to_string())?;
    let start = Instant::now();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let is_encrypted = peek_file(&source_path).unwrap_or(false);
        
        if is_encrypted {
            decrypt_file_from_path(source_path, key)
        } else {
            encrypt_file_from_path(source_path, key)
        }

    })
    .await
    .map_err(|e| e.to_string())?;
    let duration = start.elapsed();
    println!("Processing time: {:?}", duration);
    result
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
fn pick_input_file() -> Option<String> {
    FileDialog::new()
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
async fn _tauri_pick_input_file(app: tauri::AppHandle<impl tauri::Runtime>) -> Result<std::fs::File, anyhow::Error> {
    // Pick files to read and write
    let api = app.android_fs_async();
    
    // Pick files to read and write
    let selected_files = api
        .file_picker()
        .pick_files(
            None, // Initial location
            &["*/*"], // Target MIME types
            false, // If true, only files on local device
        )
        .await?;

    if selected_files.is_empty() {
        bail!("")
    }
    else {
        Ok(api.open_file_readable(&selected_files.get(0).unwrap()).await?)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_android_fs::init())
        .invoke_handler(tauri::generate_handler![
            peek_file_from_path,
            process_file_from_path,
            pick_input_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
