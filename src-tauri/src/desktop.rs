use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;
use anyhow::{anyhow, bail};
use rfd::FileDialog;
use yuralock::crypto::{AEStream, BlakeRead, CIPHERT_SIZE};
use yuralock::pubapi::{filter_fake_header, gen_dest_name};
use yuralock::EncryHeader;

use crate::{
    CHECK_BYTES, CryptoResult, FAKE_HEADER_BYTES, compatible_encrypt, copy_with_progress, emit_frontend_progress
};

pub fn encrypt_file_from_path<P: AsRef<Path>>(
    app: &tauri::AppHandle,
    source_path: P,
    key: String,
    encrypt_part: u64,
) -> Result<CryptoResult, anyhow::Error> {
    let source = File::open(&source_path)?;
    let source_size = source.metadata()?.len();

    let mut output_path = output_dir_from_input(source_path.as_ref())?;
    output_path.push(&gen_dest_name());

    let dest = File::create(&output_path)?;
    let mut processed = 0u64;
    let encrypt_size = (encrypt_part as f64 * 0.01 * source_size as f64) as u64;
    compatible_encrypt(
        source,
        dest,
        source_path
            .as_ref()
            .file_name()
            .map(|osname| osname.to_string_lossy().to_string())
            .ok_or(anyhow!("转换源文件路径出错: {:?}", source_path.as_ref()))?,
        source_size,
        encrypt_part,
        key,
        |delta| {
            processed = processed.saturating_add(delta);
            let _ = emit_frontend_progress(app, (processed * 100 / encrypt_size) as u8);
        },
    )?;

    Ok(CryptoResult {
        output_path: output_path.to_string_lossy().to_string(),
        message: "加密成功".to_string(),
    })
}

pub fn decrypt_file_from_path<P: AsRef<Path>>(
    app: &tauri::AppHandle,
    source_path: P,
    key: String,
) -> Result<CryptoResult, anyhow::Error> {
    let origin_size = fs::metadata(&source_path)?.len();
    let mut source = BlakeRead::from_read(File::open(&source_path)?);
    let mut processed = 0u64;

    filter_fake_header(&mut source)?;
    processed += FAKE_HEADER_BYTES + CHECK_BYTES;

    let encry_part: EncryHeader = EncryHeader::new(&mut source, &key)?;
    processed += encry_part.complate_header_size();
    let mut output_path = output_dir_from_input(source_path.as_ref())?;
    output_path.push(&encry_part.file_name);

    let mut dest = File::create(&output_path)?;

    {
        let encrypted_part_size = encry_part.complate_encry_size();
        let decrypt_source = source.by_ref().take(encrypted_part_size);
        let mut stream = AEStream::new(decrypt_source, &mut dest)?;
        stream
            .set_decryptor(&key)?;

        loop {
            emit_frontend_progress(app, (processed * 100 / encrypted_part_size) as u8)?;
            let next = stream
                .next()?;
            if next == usize::MAX {
                processed += CIPHERT_SIZE as u64;
                continue;
            }
            stream
                .finalize(next)?;
            break;
        }
    }

    let mut no_encry_source = source
        .by_ref()
        .take(encry_part.complate_origin_size(origin_size));
    copy_with_progress(&mut no_encry_source, &mut dest)?;

    if !source.hashcheck(&key).unwrap_or(false) {
        bail!("与原始文件校验不一致")
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
    app: &tauri::AppHandle,
    input_path: String,
    isencry: bool,
    key: String,
    encrypt_part: u64,
) -> Result<CryptoResult, String> {
    let start = Instant::now();
    let result = if isencry {
        decrypt_file_from_path(app, &input_path, key)
        .map_err(|e| e.to_string())
    } else {
        encrypt_file_from_path(app, &input_path, key, encrypt_part)
        .map_err(|e| e.to_string())
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
