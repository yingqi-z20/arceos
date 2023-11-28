use crate::sha1::sha1;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use axerrno::{AxError, AxResult};
use axfs::fops::{File, OpenOptions};
use log::debug;
use permission::permission::{current_uid, set_current_uid};

pub struct UserInfo {
    pub username: String,
    pub password: String,
    pub uid: u32,
    pub gid: u32,
    pub comment: String,
    pub home: String,
    pub shell: String,
}

fn user_list() -> AxResult<BTreeMap<u32, UserInfo>> {
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
    let lines: Vec<&str> = content.split('\n').collect();
    let mut info = BTreeMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let v: Vec<&str> = line.split(':').collect();
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

pub fn user_name(uid: u32) -> String {
    let ul = user_list();
    if let Err(_e) = ul {
        debug!("{}", _e);
        "".to_string()
    } else if let Some(ui) = ul.unwrap().get(&uid) {
        ui.username.clone()
    } else {
        "".to_string()
    }
}

pub fn user_id(name: String) -> u32 {
    let ul = user_list();
    if let Err(_e) = ul {
        debug!("{}", _e);
        0
    } else {
        for u in ul.unwrap() {
            if u.1.username.clone() == name {
                return u.0;
            }
        }
        0
    }
}

pub fn is_sudoer(name: String) -> bool {
    let cuid = current_uid().unwrap();
    set_current_uid(0).unwrap();
    let mut opt = OpenOptions::new();
    opt.read(true);
    let file_result = File::open("/etc/sudoers", &opt);
    if file_result.is_err() {
        set_current_uid(cuid).unwrap();
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
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let v: Vec<&str> = line.trim().split(' ').collect();
        if v[0].to_string() == name {
            set_current_uid(cuid).unwrap();
            return true;
        }
    }
    set_current_uid(cuid).unwrap();
    false
}

pub fn verify(uid: u32, password: String) -> bool {
    let cuid = current_uid().unwrap();
    set_current_uid(0).unwrap();
    let mut opt = OpenOptions::new();
    opt.read(true);
    let file_result = File::open("/etc/shadow", &opt);
    if file_result.is_err() {
        set_current_uid(cuid).unwrap();
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
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let v: Vec<&str> = line.split(':').collect();
        if v[0].to_string() == uid.to_string() {
            set_current_uid(cuid).unwrap();
            let s1 = sha1(&password.as_str());
            let mut s1h = String::new();
            for i in s1 {
                s1h.push(char::from((i / 16) + b'a'));
                s1h.push(char::from((i % 16) + b'a'));
            }
            return v.len() <= 1 || s1h == v[1].to_string();
        }
    }
    set_current_uid(cuid).unwrap();
    true
}

pub fn set_password(password: String) -> AxResult {
    let cuid = current_uid().unwrap();
    set_current_uid(0).unwrap();
    let mut opt = OpenOptions::new();
    opt.read(true);
    let mut file = File::open("/etc/shadow", &opt)?;
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
    let mut new_lines: Vec<String> = Vec::new();
    let s1 = sha1(&password.as_str());
    let mut s1h = String::new();
    for i in s1 {
        s1h.push(char::from((i / 16) + b'a'));
        s1h.push(char::from((i % 16) + b'a'));
    }
    new_lines.push(cuid.to_string() + ":" + s1h.as_str());
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let v: Vec<&str> = line.split(':').collect();
        if v[0].to_string() == cuid.to_string() {
            continue;
        } else {
            new_lines.push(line.to_string())
        }
    }
    let mut opt = OpenOptions::new();
    opt.write(true);
    opt.create(true);
    opt.truncate(true);
    let mut file = File::open("/etc/shadow", &opt)?;
    for new_line in new_lines {
        file.write((new_line + "\n").as_bytes())?;
    }
    set_current_uid(cuid).unwrap();
    Ok(())
}
