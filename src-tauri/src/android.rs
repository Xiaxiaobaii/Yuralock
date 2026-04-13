use std::io::Read;

use anyhow::bail;
use tauri_plugin_android_fs::{AndroidFsExt, FileUri};
use yuralock::{
    crypto::{AEStream, BlakeRead, CIPHERT_SIZE},
    pubapi::{filter_fake_header, peek_source, gen_dest_name},
    EncryHeader,
};

use crate::{
    CHECK_BYTES, CryptoResult, FAKE_HEADER_BYTES, compatible_encrypt, copy_with_progress, emit_frontend_progress
};

pub(crate) async fn peek_file_from_uri(app: &tauri::AppHandle, input_path: &str) -> bool {
    let source_uri = FileUri::from_uri(input_path);
    let api = app.android_fs_async();
    let source_file = api.open_file_readable(&source_uri).await;
    if let Ok(mut file) = source_file {
        peek_source(&mut file)
    } else {
        false
    }
}

pub(crate) async fn pick_input_file(app: &tauri::AppHandle) -> String {
    let source_uri = pick_android_input_and_output_dir(app).await;
    if let Ok(uri) = source_uri {
        uri.uri
    } else {
        String::new()
    }
}

async fn encrypt_android_uri(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    source_uri: &FileUri,
    output_dir_uri: &FileUri,
    key: String,
    encrypt_part: u64,
) -> Result<CryptoResult, String> {
    let api = app.android_fs_async();
    let source = api
        .open_file_readable(source_uri)
        .await
        .map_err(|e| e.to_string())?;
    let source_size = api.get_len(source_uri).await.map_err(|e| e.to_string())?;
    let source_name = api.get_name_or_last_path_segment(source_uri).await;

    let target_uri = api
        .create_new_file(output_dir_uri, &gen_dest_name(), None)
        .await
        .map_err(|e| e.to_string())?;
    let target_file = api
        .open_file_writable(&target_uri)
        .await
        .map_err(|e| e.to_string())?;
    let mut processed = 0u64;
    let encrypt_size = (encrypt_part as f64 * 0.01 * source_size as f64) as u64;
    compatible_encrypt(
        source,
        target_file,
        source_name,
        source_size,
        encrypt_part,
        key,
        |delta| {
            processed = processed.saturating_add(delta);
            let _ = emit_frontend_progress(app, (processed * 100 / encrypt_size) as u8);
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(CryptoResult {
        output_path: target_uri.uri,
        message: "加密成功".to_string(),
    })
}

async fn decrypt_android_uri(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    source_uri: &FileUri,
    output_dir_uri: &FileUri,
    key: String,
) -> Result<CryptoResult, anyhow::Error> {
    let api = app.android_fs_async();
    let origin_size = api.get_len(source_uri).await?;
    let source_file = api
        .open_file_readable(source_uri)
        .await?;
    let mut source = BlakeRead::from_read(source_file);
    let mut processed = 0u64;

    filter_fake_header(&mut source)?;
    processed += FAKE_HEADER_BYTES + CHECK_BYTES;
    let encry_part: EncryHeader = EncryHeader::new(&mut source, &key)?;
    processed += encry_part.complate_header_size();
    let target_uri = api
        .create_new_file(output_dir_uri, &encry_part.file_name, None)
        .await?;
    let mut dest = api
        .open_file_writable(&target_uri)
        .await?;

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
    copy_with_progress(&mut no_encry_source, &mut dest);

    if !source.hashcheck(&key).unwrap_or(false) {
       bail!("文件校验失败，与原始文件不一致") 
    }

    Ok(CryptoResult {
        output_path: target_uri.uri,
        message: "解密成功".to_string(),
    })
}

pub(crate) async fn process_file_from_android_uri(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    input_path: &str,
    isencry: bool,
    key: String,
    encrypt_part: u64,
) -> Result<CryptoResult, String> {
    let source_uri = FileUri::from_uri(input_path);
    let output_dir_path = input_path.split('/').collect::<Vec<&str>>();
    let output_dir_path = output_dir_path[0..output_dir_path.len() - 1].join("/");
    let output_dir_uri = FileUri::from_uri(output_dir_path);
    let output_dir_uri = ensure_permitted_output_dir_uri(app, output_dir_uri).await?;

    if isencry {
        decrypt_android_uri(app, &source_uri, &output_dir_uri, key).await
        .map_err(|err| err.to_string())
    } else {
        encrypt_android_uri(app, &source_uri, &output_dir_uri, key, encrypt_part).await
    }
}

async fn pick_android_input_and_output_dir(
    app: &tauri::AppHandle<impl tauri::Runtime>,
) -> Result<FileUri, anyhow::Error> {
    let api = app.android_fs_async();

    let selected_file = api
        .file_picker()
        .pick_file(
            None,     // Initial location
            &["*/*"], // Target MIME types
            false,    // If true, only files on local device
        )
        .await?;

    let Some(source_uri) = selected_file else {
        bail!("文件选择器无返回文件")
    };

    Ok(source_uri)
}

pub(crate) async fn ensure_permitted_output_dir_uri(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    output_dir_uri: FileUri,
) -> Result<FileUri, String> {
    // Already a tree-backed URI from a directory picker, keep it.
    if output_dir_uri.document_top_tree_uri.is_some() {
        return Ok(output_dir_uri);
    }

    let api = app.android_fs_async();
    let picked = api
        .file_picker()
        .pick_dir(Some(&output_dir_uri), false)
        .await
        .map_err(|e| e.to_string())?;

    picked.ok_or_else(|| "未授予输出目录写入权限，请重新选择目录".to_string())
}
