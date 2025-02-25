use alloc::string::String;
use axerrno::AxError::PermissionDenied;
use axerrno::{ax_err, AxError, AxResult};
use axfs::api::current_uid;
use axfs::fops::{Directory, File, FileAttr, OpenOptions};

pub use axfs::fops::DirEntry as AxDirEntry;
pub use axfs::fops::FileAttr as AxFileAttr;
pub use axfs::fops::FilePerm as AxFilePerm;
pub use axfs::fops::FileType as AxFileType;
pub use axfs::fops::OpenOptions as AxOpenOptions;
pub use axio::SeekFrom as AxSeekFrom;

#[cfg(feature = "myfs")]
pub use axfs::fops::{Disk as AxDisk, MyFileSystemIf};
#[cfg(feature = "user")]
use axuser::api::user_name;

/// A handle to an opened file.
pub struct AxFileHandle(File);

/// A handle to an opened directory.
pub struct AxDirHandle(Directory);

pub fn ax_open_file(path: &str, opts: &AxOpenOptions) -> AxResult<AxFileHandle> {
    Ok(AxFileHandle(File::open(path, opts)?))
}

pub fn ax_open_dir(path: &str, opts: &AxOpenOptions) -> AxResult<AxDirHandle> {
    Ok(AxDirHandle(Directory::open_dir(path, opts)?))
}

pub fn ax_read_file(file: &mut AxFileHandle, buf: &mut [u8]) -> AxResult<usize> {
    file.0.read(buf)
}

pub fn ax_read_file_at(file: &AxFileHandle, offset: u64, buf: &mut [u8]) -> AxResult<usize> {
    file.0.read_at(offset, buf)
}

pub fn ax_write_file(file: &mut AxFileHandle, buf: &[u8]) -> AxResult<usize> {
    file.0.write(buf)
}

pub fn ax_write_file_at(file: &AxFileHandle, offset: u64, buf: &[u8]) -> AxResult<usize> {
    file.0.write_at(offset, buf)
}

pub fn ax_truncate_file(file: &AxFileHandle, size: u64) -> AxResult {
    file.0.truncate(size)
}

pub fn ax_flush_file(file: &AxFileHandle) -> AxResult {
    file.0.flush()
}

pub fn ax_seek_file(file: &mut AxFileHandle, pos: AxSeekFrom) -> AxResult<u64> {
    file.0.seek(pos)
}

pub fn ax_file_attr(file: &AxFileHandle) -> AxResult<AxFileAttr> {
    file.0.get_attr()
}

pub fn ax_file_change_attr(file: &AxFileHandle, perm: u16, uid: u32, gid: u32) -> AxResult {
    let a = file.0.get_attr()?;
    if current_uid().is_ok_and(|uid| (uid != 0 && uid != a.user_id())) {
        return Err(PermissionDenied);
    }
    let new_attr = FileAttr::new(
        a.perm_from_u16(perm),
        uid,
        gid,
        a.file_type(),
        a.size(),
        a.blocks(),
    );
    file.0.set_attr(new_attr)
}

pub fn ax_read_dir(dir: &mut AxDirHandle, dirents: &mut [AxDirEntry]) -> AxResult<usize> {
    dir.0.read_dir(dirents)
}

pub fn ax_create_dir(path: &str) -> AxResult {
    axfs::api::create_dir(path)
}

pub fn ax_remove_dir(path: &str) -> AxResult {
    axfs::api::remove_dir(path)
}

pub fn ax_remove_file(path: &str) -> AxResult {
    axfs::api::remove_file(path)
}

pub fn ax_rename(old: &str, new: &str) -> AxResult {
    axfs::api::rename(old, new)
}

pub fn ax_current_dir() -> AxResult<String> {
    axfs::api::current_dir()
}

pub fn ax_set_current_dir(path: &str) -> AxResult {
    axfs::api::set_current_dir(path)
}

pub fn ax_getuid() -> AxResult<u32> {
    current_uid()
}

#[allow(unreachable_code)]
pub fn ax_setuid(uid: u32) -> AxResult {
    if current_uid().is_ok_and(|uid| (uid == 0)) {
        return axfs::api::set_current_uid(uid);
    }
    #[cfg(feature = "user")]
    {
        axhal::console::putchar(b'P');
        axhal::console::putchar(b'a');
        axhal::console::putchar(b's');
        axhal::console::putchar(b's');
        axhal::console::putchar(b'w');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b'd');
        axhal::console::putchar(b':');
        axhal::console::putchar(b' ');
        let password = get_password();
        if !axuser::api::verify(uid, password) {
            return Err(AxError::AuthenticationFailure);
        }
        return axfs::api::set_current_uid(uid);
    }
    Err(PermissionDenied)
}

#[allow(unreachable_code)]
pub fn sudo() -> AxResult {
    let uid = current_uid()?;
    if uid == 0 {
        return Ok(());
    }
    #[cfg(not(feature = "user"))]
    { return Err(PermissionDenied); }
    #[cfg(feature = "user")]
    {
        let name = user_name(uid);
        axhal::console::putchar(b'[');
        axhal::console::putchar(b's');
        axhal::console::putchar(b'u');
        axhal::console::putchar(b'd');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b']');
        axhal::console::putchar(b' ');
        axhal::console::putchar(b'p');
        axhal::console::putchar(b'a');
        axhal::console::putchar(b's');
        axhal::console::putchar(b's');
        axhal::console::putchar(b'w');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b'd');
        axhal::console::putchar(b' ');
        axhal::console::putchar(b'f');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b' ');

        for c in name.as_str().as_bytes() {
            axhal::console::putchar(*c);
        }

        axhal::console::putchar(b' ');
        axhal::console::putchar(b':');
        axhal::console::putchar(b' ');
        let password = get_password();
        if axuser::api::is_sudoer(name) {
            if !axuser::api::verify(uid, password) {
                return Err(AxError::AuthenticationFailure);
            } else {
                axfs::api::set_current_uid(0)
            }
        } else {
            Err(PermissionDenied)
        }
    }
}

#[allow(unreachable_code)]
pub fn ax_setpassword() -> AxResult {
    #[cfg(feature = "user")]
    {
        let name = user_name(current_uid().unwrap_or(0));
        axhal::console::putchar(b'C');
        axhal::console::putchar(b'h');
        axhal::console::putchar(b'a');
        axhal::console::putchar(b'n');
        axhal::console::putchar(b'g');
        axhal::console::putchar(b'i');
        axhal::console::putchar(b'n');
        axhal::console::putchar(b'g');
        axhal::console::putchar(b' ');
        axhal::console::putchar(b'p');
        axhal::console::putchar(b'a');
        axhal::console::putchar(b's');
        axhal::console::putchar(b's');
        axhal::console::putchar(b'w');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b'd');
        axhal::console::putchar(b' ');
        axhal::console::putchar(b'f');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b' ');

        for c in name.as_str().as_bytes() {
            axhal::console::putchar(*c);
        }

        axhal::console::putchar(b'.');
        axhal::console::putchar(b'\n');
        axhal::console::putchar(b'C');
        axhal::console::putchar(b'u');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b'e');
        axhal::console::putchar(b'n');
        axhal::console::putchar(b't');
        axhal::console::putchar(b' ');
        axhal::console::putchar(b'p');
        axhal::console::putchar(b'a');
        axhal::console::putchar(b's');
        axhal::console::putchar(b's');
        axhal::console::putchar(b'w');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b'd');
        axhal::console::putchar(b':');
        axhal::console::putchar(b' ');
        let password = get_password();
        if !axuser::api::verify(current_uid().unwrap_or(0), password) {
            return Err(AxError::AuthenticationFailure);
        }
        axhal::console::putchar(b'N');
        axhal::console::putchar(b'e');
        axhal::console::putchar(b'w');
        axhal::console::putchar(b' ');
        axhal::console::putchar(b'p');
        axhal::console::putchar(b'a');
        axhal::console::putchar(b's');
        axhal::console::putchar(b's');
        axhal::console::putchar(b'w');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b'd');
        axhal::console::putchar(b':');
        axhal::console::putchar(b' ');
        let password1 = get_password();
        axhal::console::putchar(b'R');
        axhal::console::putchar(b'e');
        axhal::console::putchar(b't');
        axhal::console::putchar(b'y');
        axhal::console::putchar(b'p');
        axhal::console::putchar(b'e');
        axhal::console::putchar(b' ');
        axhal::console::putchar(b'n');
        axhal::console::putchar(b'e');
        axhal::console::putchar(b'w');
        axhal::console::putchar(b' ');
        axhal::console::putchar(b'p');
        axhal::console::putchar(b'a');
        axhal::console::putchar(b's');
        axhal::console::putchar(b's');
        axhal::console::putchar(b'w');
        axhal::console::putchar(b'o');
        axhal::console::putchar(b'r');
        axhal::console::putchar(b'd');
        axhal::console::putchar(b':');
        axhal::console::putchar(b' ');
        let password2 = get_password();
        if password1 == password2 {
            return axuser::api::set_password(password1);
        }
        let pnm = "Sorry, passwords do not match.\n";
        for c in pnm.as_bytes() {
            axhal::console::putchar(*c);
        }
        return Err(AxError::AuthenticationFailure);
    }
    Err(PermissionDenied)
}

#[allow(unused)]
fn get_password() -> String {
    let mut password = String::new();
    const DL: u8 = b'\x7f';
    const BS: u8 = b'\x08';
    loop {
        if let Some(c) = axhal::console::getchar().map(|c| if c == b'\r' { b'\n' } else { c }) {
            if c == b'\n' {
                break;
            }
            if c == DL || c == BS {
                password.pop();
                continue;
            }
            password.push(c as char);
        }
    }
    axhal::console::putchar(b'\n');
    password
}
