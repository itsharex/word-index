use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
    time::SystemTime, io::Read,
};

use anyhow::*;
use encoding::{Encoding, DecoderTrap};
use encoding::all::GBK;
use serde::{Deserialize, Serialize};
use tauri::api::process::Command;
use tracing::{error, info, instrument};

const PLAIN_FILE_TYPE: [&str; 2] = ["txt", "sql"];
const HYPER_FILE_TYPE: [&str; 2] = ["docx", "doc"];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Docx {
    id: String,
    name: String,
    path: String,
    content: String,
    timestamp: u64,
}

impl Docx {
    pub fn new(path: &PathBuf) -> Result<Docx> {
        if !is_support(path) {
            return Err(anyhow!("未支持的文件类型！"));
        }
        let name = path.file_name().unwrap().to_str().unwrap();
        let path_name = path.to_str().unwrap();
        let timestamp = get_file_timestamp(&path)?;
        let md5 = md5::compute(path_name.as_bytes());
        let id = format!("{:x}", md5);
        Ok(Self {
            id,
            name: name.to_string(),
            path: path_name.to_string(),
            content: String::new(),
            timestamp,
        })
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn set_content(&mut self) -> Result<()> {
        let path = PathBuf::from(&self.path);
        let extension = path.extension().unwrap();
        if is_plain(extension) {
            self.content = read_plain_file(&self.path)?;
        } else {
            self.content = read_docx_file(&self.path)?;
        }
        Ok(())
    }
}

pub fn is_support(path: &PathBuf) -> bool {
    let extension = path.extension();
    if let Some(e) = extension {
        is_plain(e) || is_hyper(e)
    } else {
        false
    }
}

fn is_plain(extension: &OsStr) -> bool {
    for e in PLAIN_FILE_TYPE {
        if e == extension {
            return true;
        }
    }
    false
}

fn is_hyper(extension: &OsStr) -> bool {
    for e in HYPER_FILE_TYPE {
        if e == extension {
            return true;
        }
    }
    false
}

/// 文件时间戳
fn get_file_timestamp(path: &Path) -> Result<u64> {
    let file = File::open(path)?;
    let timestamp = file
        .metadata()?
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    Ok(timestamp)
}

/// 调用pandoc，读取docx文件，返回文件内容
/// 需要提前安装pandoc
#[instrument]
fn read_docx_file(path: &str) -> Result<String> {
    info!("read_docx_file");
    Command::new("pandoc")
        .args(&[
            "-t",
            "plain",
            "--wrap=none",
            "--markdown-headings=atx",
            path,
        ])
        .output()
        .map(|output| output.stdout)
        .map_err(|e| e.into())
}

#[instrument]
fn read_plain_file(path: &str) -> Result<String> {
    match std::fs::read_to_string(path) {
        Err(e) => {
            error!("读取文件{}失败：{}", path, e);
            let mut file = File::open(path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            GBK.decode(&buf, DecoderTrap::Strict).map_err(|e| {
                error!("读取文件{}失败：{}", path, e);
                anyhow!(e)
            })
        },
        r => r.map_err(|e| anyhow!(e)),
    }
}
