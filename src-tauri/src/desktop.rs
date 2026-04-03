use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

use rfd::FileDialog;
use uuid::Uuid;
use yuralock::crypto::{decrypt, BlakeRead};
use yuralock::pubapi::filter_fake_header;
use yuralock::EncryHeader;

use crate::{compatible_encrypt, CryptoResult};

pub fn encrypt_file_from_path(
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

pub fn decrypt_file_from_path(source_path: PathBuf, key: String) -> Result<CryptoResult, String> {
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

pub fn pick_input_file() -> String {
    FileDialog::new()
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub async fn process_file_from_path_inner(
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

fn validate_common(input_path: &str, key: &str) -> Result<PathBuf, String> {
    let source_path = normalize_input_path(input_path)?;
    if key.is_empty() {
        return Err("请输入密钥".to_string());
    }
    Ok(source_path)
}

fn normalize_input_path(input_path: &str) -> Result<PathBuf, String> {
    let trimmed = input_path.trim();
    if trimmed.is_empty() {
        return Err("请选择文件".to_string());
    }
    Ok(PathBuf::from(trimmed))
}

fn output_dir_from_input(input_path: &Path) -> Result<PathBuf, String> {
    match input_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => Ok(parent.to_path_buf()),
        _ => std::env::current_dir().map_err(|e| e.to_string()),
    }
}
