use std::{
    fs::{self, File},
    io::{self, Read},
    path::PathBuf,
};

use rfd::FileDialog;
use serde::Serialize;
use tauri::Emitter;
use uuid::Uuid;
use yuralock::{
    EncryFile, EncryHeader, crypto::{BlakeRead, decrypt, encrypt}, hashdrop
};

#[derive(Serialize)]
struct CryptoResult {
    output_path: String,
    message: String,
}

fn emit_log(window: &tauri::Window, message: impl Into<String>) {
    let _ = window.emit("crypto-log", message.into());
}

fn normalize_output_dir(output_dir: &str) -> Result<PathBuf, String> {
    if output_dir.trim().is_empty() {
        std::env::current_dir().map_err(|e| e.to_string())
    } else {
        Ok(PathBuf::from(output_dir.trim()))
    }
}

fn validate_common(input_path: &str, key: &str) -> Result<(), String> {
    if input_path.trim().is_empty() {
        return Err("请输入源文件路径".to_string());
    }
    if key.is_empty() {
        return Err("请输入密钥".to_string());
    }
    Ok(())
}

fn read_exact_from_normal(source: &mut BlakeRead<File>, buf: &mut [u8]) -> Result<(), String> {
    let mut offset = 0;
    while offset < buf.len() {
        let read = source
            .normal_read(&mut buf[offset..])
            .map_err(|e| e.to_string())?;
        if read == 0 {
            return Err("读取文件校验码失败".to_string());
        }
        offset += read;
    }
    Ok(())
}

#[tauri::command]
fn encrypt_part_file_from_path(
    window: tauri::Window,
    input_path: String,
    output_dir: String,
    part: u64,
    key: String,
) -> Result<CryptoResult, String> {
    validate_common(&input_path, &key)?;

    if part > 100 {
        return Err("加密比例必须在 0-100 之间".to_string());
    }

    emit_log(&window, "开始加密...");

    let source_path = PathBuf::from(input_path.trim());
    let mut source = File::open(&source_path).map_err(|e| e.to_string())?;
    emit_log(&window, "源文件已打开");

    let mut output_path = normalize_output_dir(&output_dir)?;
    let file_name = Uuid::new_v4().to_string();
    output_path.push(&file_name);

    let mut dest = EncryFile::new(output_path.clone(), key.clone()).map_err(|e| e.to_string())?;
    emit_log(&window, "目标文件已创建");

    let encrypt_part_size = dest
        .write_header(&source_path, part)
        .map_err(|e| e.to_string())?;
    emit_log(&window, "文件头和伪装层写入完毕");

    encrypt(
        &mut source
            .try_clone()
            .map_err(|e| e.to_string())?
            .take(encrypt_part_size),
        &mut dest,
        &key,
    )
    .map_err(|e| e.to_string())?;
    emit_log(&window, "加密部分写入完毕");

    io::copy(&mut source, &mut dest).map_err(|e| e.to_string())?;
    emit_log(&window, "未加密部分拷贝完毕");

    dest.finilaize().map_err(|e| e.to_string())?;
    emit_log(&window, "文件校验信息写入完毕");

    Ok(CryptoResult {
        output_path: output_path.to_string_lossy().to_string(),
        message: "加密成功".to_string(),
    })
}

#[tauri::command]
fn decrypt_part_file_from_path(
    window: tauri::Window,
    input_path: String,
    output_dir: String,
    key: String,
) -> Result<CryptoResult, String> {
    validate_common(&input_path, &key)?;

    emit_log(&window, "开始解密...");

    let source_path = PathBuf::from(input_path.trim());
    let origin_size = fs::metadata(&source_path).map_err(|e| e.to_string())?.len();
    let mut source = BlakeRead::from_read(File::open(&source_path).map_err(|e| e.to_string())?);
    emit_log(&window, "源文件已打开");

    let mut fake_header: [u8; 8] = [0; 8];
    source
        .read_exact(&mut fake_header)
        .map_err(|_| "伪装层读取失败")?;

    let encry_part: EncryHeader = EncryHeader::new(&mut source, &key).map_err(|_| "读取文件头失败! 文件缺损或密钥不正确！")?;
    emit_log(&window, format!("文件头解析完成, 文件加密比例: {:.2}%", encry_part.encry_byte_number as f64 / (encry_part.complate_origin_size(origin_size) + encry_part.encry_byte_number) as f64 * 100.0 ));

    let mut output_path = normalize_output_dir(&output_dir)?;
    if !output_path.is_file() {
        output_path.push(&encry_part.file_name);
    }

    let mut limit_source = source.by_ref().take(encry_part.complate_encry_size());
    let mut dest = File::create(&output_path).map_err(|e| e.to_string())?;

    decrypt(&mut limit_source, &mut dest, &key).map_err(|_| "解密失败! 文件缺损或密钥不正确！")?;
    emit_log(&window, "加密部分解密完成");

    let mut no_encry_source = source.by_ref().take(encry_part.complate_origin_size(origin_size));
    io::copy(&mut no_encry_source, &mut dest).map_err(|_| "原始内容拷贝失败！")?;
    emit_log(&window, "原始内容拷贝完成");

    let mut hash_buf = vec![0u8; 16];
    read_exact_from_normal(&mut source, &mut hash_buf)?;

    if hashdrop(source.update_finalize(), &key) != *hash_buf {
        emit_log(&window, "文件校验失败");
        return Err("文件校验失败，还原结果与原始文件不一致".to_string());
    }

    emit_log(&window, "文件校验通过");

    Ok(CryptoResult {
        output_path: output_path.to_string_lossy().to_string(),
        message: "解密成功".to_string(),
    })
}

#[tauri::command]
fn pick_input_file() -> Option<String> {
    FileDialog::new()
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_output_dir() -> Option<String> {
    FileDialog::new()
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            encrypt_part_file_from_path,
            decrypt_part_file_from_path,
            pick_input_file,
            pick_output_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
