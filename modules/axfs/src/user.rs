use crate::fops::{File, OpenOptions};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use axerrno::{AxError, AxResult};
use axsync::Mutex;

static UID: Mutex<u32> = Mutex::new(2);

pub(crate) fn current_uid() -> AxResult<u32> {
    Ok(*UID.lock())
}

pub(crate) fn current_gid() -> AxResult<u32> {
    current_uid()
}

pub(crate) fn set_current_uid(uid: u32) -> AxResult {
    *UID.lock() = uid;
    Ok(())
}

pub struct UserInfo {
    username: String,
    password: String,
    uid: u32,
    gid: u32,
    comment: String,
    home: String,
    shell: String,
}

pub(crate) fn user_list() -> AxResult<BTreeMap<u32, UserInfo>> {
    let mut opt = OpenOptions::new();
    opt.read(true);
    let mut file = File::open("/etc/passwd", &opt)?;
    let mut content = String::new();
    loop {
        let mut buf = [0u8; 256];
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        content += String::from_utf8_lossy(&(buf[..n])).as_ref();
    }
    debug!("{}", content);
    let lines: Vec<&str> = content.split('\n').collect();
    let mut info = BTreeMap::new();
    for line in lines {
        let v: Vec<&str> = line.split('\n').collect();
        if v.len() < 7 {
            return Err(AxError::InvalidData);
        }
        let uid = String::from(v[2]).parse::<u32>().unwrap_or(u32::MAX);
        if uid == u32::MAX {
            return Err(AxError::InvalidData);
        }
        let gid = String::from(v[3]).parse::<u32>().unwrap_or(u32::MAX);
        if gid == u32::MAX {
            return Err(AxError::InvalidData);
        }
        info.insert(
            uid,
            UserInfo {
                username: v[0].to_string(),
                password: v[1].to_string(),
                uid,
                gid,
                comment: v[4].to_string(),
                home: v[5].to_string(),
                shell: v[6].to_string(),
            },
        );
    }
    Ok(info)
}

pub(crate) fn verify(uid: u32, password: String) -> bool {
    let mut opt = OpenOptions::new();
    opt.read(true);
    let mut file_result = File::open("/etc/passwd", &opt);
    if file_result.is_err() {
        return false;
    }
    let mut file = file_result.unwrap();
    let mut content = String::new();
    loop {
        let mut buf = [0u8; 256];
        let n = file.read(&mut buf);
        if n.is_err() || n.is_ok_and(|x| x == 0) {
            break;
        }
        content += String::from_utf8_lossy(&(buf[..n.unwrap()])).as_ref();
    }
    let lines: Vec<&str> = content.split('\n').collect();
    false
}
