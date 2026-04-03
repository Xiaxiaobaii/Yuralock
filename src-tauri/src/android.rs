use std::io::{self, Read};

use anyhow::bail;
use tauri_plugin_android_fs::{AndroidFsExt, FileUri};
use uuid::Uuid;
use yuralock::{
    crypto::{decrypt, encrypt, BlakeRead},
    pubapi::{filter_fake_header, peek_source},
    EncryFile, EncryHeader,
};

use crate::{compatible_encrypt, CryptoResult, DEFAULT_ENCRYPT_PART};

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

pub(crate) async fn pick_input_file(app: &tauri::AppHandle) -> Result<String, String> {
    let source_uri = pick_android_input_and_output_dir(app)
        .await
        .map_err(|e| e.to_string())?;
    Ok(source_uri.uri)
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

    //let source_size = api.get_len(source_uri).await.map_err(|e| e.to_string())?;
    let source_name =
        sanitize_android_file_name(&api.get_name_or_last_path_segment(source_uri).await);
    let output_name = Uuid::new_v4().to_string();

    let target_uri = api
        .create_new_file(output_dir_uri, &output_name, None)
        .await
        .map_err(|e| e.to_string())?;
    let target_file = api
        .open_file_writable(&target_uri)
        .await
        .map_err(|e| e.to_string())?;

    compatible_encrypt(source, target_file, source_name, encrypt_part, key)
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
) -> Result<CryptoResult, String> {
    let api = app.android_fs_async();
    let origin_size = api.get_len(source_uri).await.map_err(|e| e.to_string())?;
    let source_file = api
        .open_file_readable(source_uri)
        .await
        .map_err(|e| e.to_string())?;
    let mut source = BlakeRead::from_read(source_file);

    filter_fake_header(&mut source).map_err(|_| "伪装层读取失败".to_string())?;
    let encry_part: EncryHeader = EncryHeader::new(&mut source, &key)
        .map_err(|_| "读取文件头失败，文件损坏或密钥错误".to_string())?;
    let output_name = sanitize_android_file_name(&encry_part.file_name);
    let target_uri = api
        .create_new_file(output_dir_uri, &output_name, None)
        .await
        .map_err(|e| e.to_string())?;
    let mut dest = api
        .open_file_writable(&target_uri)
        .await
        .map_err(|e| e.to_string())?;

    let mut limit_source = source.by_ref().take(encry_part.complate_encry_size());
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
    } else {
        encrypt_android_uri(app, &source_uri, &output_dir_uri, key, encrypt_part).await
    }
}

fn sanitize_android_file_name(name: &str) -> String {
    let sanitized = name
        .trim()
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "picked-file.bin".to_string()
    } else {
        sanitized
    }
}

fn strip_android_uuid_prefix(name: &str) -> String {
    if let Some((prefix, rest)) = name.split_once('_') {
        if !rest.is_empty() {
            return rest.to_string();
        }
    }
    name.to_string()
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
