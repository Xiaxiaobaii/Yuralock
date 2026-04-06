use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rfd::FileDialog;
use yuralock::crypto::{AEStream, BlakeRead, CIPHERT_SIZE};
use yuralock::pubapi::{filter_fake_header, gen_dest_name};
use yuralock::EncryHeader;

use crate::{
    compatible_encrypt, copy_with_progress, emit_progress_if_changed, CryptoResult, CHECK_BYTES,
    FAKE_HEADER_BYTES,
};

pub fn encrypt_file_from_path<P: AsRef<Path>>(
    app: &tauri::AppHandle,
    source_path: P,
    key: String,
    encrypt_part: u64,
) -> Result<CryptoResult, String> {
    let source = File::open(&source_path).map_err(|e| e.to_string())?;
    let source_size = source.metadata().map_err(|e| e.to_string())?.len();

    let mut output_path = output_dir_from_input(source_path.as_ref()).map_err(|e| e.to_string())?;
    output_path.push(&gen_dest_name());

    let dest = File::create(&output_path).map_err(|e| e.to_string())?;
    let mut processed = 0u64;
    let mut last_percent = 0u8;

    compatible_encrypt(
        source,
        dest,
        source_path
            .as_ref()
            .file_name()
            .map(|osname| osname.to_string_lossy().to_string())
            .ok_or(format!("转换源文件路径出错: {:?}", source_path.as_ref()))?,
        source_size,
        encrypt_part,
        key,
        |delta| {
            processed = processed.saturating_add(delta);
            emit_progress_if_changed(app, processed, source_size, &mut last_percent);
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(CryptoResult {
        output_path: output_path.to_string_lossy().to_string(),
        message: "加密成功".to_string(),
    })
}

pub fn decrypt_file_from_path<P: AsRef<Path>>(
    app: &tauri::AppHandle,
    source_path: P,
    key: String,
) -> Result<CryptoResult, String> {
    let origin_size = fs::metadata(&source_path).map_err(|e| e.to_string())?.len();
    let mut source = BlakeRead::from_read(File::open(&source_path).map_err(|e| e.to_string())?);
    let mut processed = 0u64;
    let mut last_percent = 0u8;

    filter_fake_header(&mut source).map_err(|_| "伪装层读取失败".to_string())?;
    processed += FAKE_HEADER_BYTES + CHECK_BYTES;
    emit_progress_if_changed(app, processed, origin_size, &mut last_percent);

    let encry_part: EncryHeader = EncryHeader::new(&mut source, &key)
        .map_err(|_| "读取文件头失败，文件损坏或密钥错误".to_string())?;
    processed += encry_part.complate_header_size();
    emit_progress_if_changed(app, processed, origin_size, &mut last_percent);
    let mut output_path = output_dir_from_input(source_path.as_ref()).map_err(|e| e.to_string())?;
    output_path.push(&encry_part.file_name);

    let mut dest = File::create(&output_path).map_err(|e| e.to_string())?;

    {
        let encrypted_part_size = encry_part.complate_encry_size();
        let decrypt_source = source.by_ref().take(encrypted_part_size);
        let mut stream = AEStream::new(decrypt_source, &mut dest).map_err(|e| e.to_string())?;
        stream
            .set_decryptor(&key)
            .map_err(|_| "读取文件头失败，文件损坏或密钥错误".to_string())?;

        loop {
            let next = stream
                .next()
                .map_err(|_| "解密失败，文件损坏或密钥错误".to_string())?;
            if next == usize::MAX {
                processed += CIPHERT_SIZE as u64;
                emit_progress_if_changed(app, processed, origin_size, &mut last_percent);
                continue;
            }
            processed += next as u64;
            emit_progress_if_changed(app, processed, origin_size, &mut last_percent);
            stream
                .finalize(next)
                .map_err(|_| "解密失败，文件损坏或密钥错误".to_string())?;
            break;
        }
    }

    let mut no_encry_source = source
        .by_ref()
        .take(encry_part.complate_origin_size(origin_size));
    copy_with_progress(&mut no_encry_source, &mut dest, &mut |delta| {
        processed += delta;
        emit_progress_if_changed(app, processed, origin_size, &mut last_percent);
    })
    .map_err(|_| "原始内容拷贝失败".to_string())?;

    if !source.hashcheck(&key).unwrap_or(false) {
        return Err("文件校验失败，与原始文件不一致".to_string());
    }
    emit_progress_if_changed(app, origin_size, origin_size, &mut last_percent);

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
    app: &tauri::AppHandle,
    input_path: String,
    isencry: bool,
    key: String,
    encrypt_part: u64,
) -> Result<CryptoResult, String> {
    let start = Instant::now();
    let result = if isencry {
        decrypt_file_from_path(app, &input_path, key)
    } else {
        encrypt_file_from_path(app, &input_path, key, encrypt_part)
    };
    let duration = start.elapsed();
    println!("Processing time: {:?}", duration);
    result
}

pub fn output_dir_from_input(input_path: &Path) -> Result<PathBuf, anyhow::Error> {
    match input_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => Ok(parent.to_path_buf()),
        _ => Ok(std::env::current_dir()?),
    }
}
