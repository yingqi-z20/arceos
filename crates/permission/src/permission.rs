use axerrno::AxResult;
use axfs_vfs::VfsNodePerm;
use axsync::Mutex;
use capability::Cap;

static UID: Mutex<u32> = Mutex::new(0);

pub fn current_uid() -> AxResult<u32> {
    Ok(*UID.lock())
}

pub fn current_gid() -> AxResult<u32> {
    current_uid()
}

pub fn set_current_uid(uid: u32) -> AxResult {
    *UID.lock() = uid;
    Ok(())
}

pub fn fops_cap(perm: VfsNodePerm, uid: u32, gid: u32) -> Cap {
    let current_uid = current_uid().unwrap();
    let current_gid = current_gid().unwrap();
    let fp = if current_uid == 0 {
        (true, true, true)
    } else if current_uid == uid && current_gid == gid {
        (
            perm.owner_readable(),
            perm.owner_writable(),
            perm.owner_executable(),
        )
    } else if current_gid == gid {
        (
            perm.group_readable(),
            perm.group_writable(),
            perm.group_executable(),
        )
    } else {
        (
            perm.other_readable(),
            perm.other_writable(),
            perm.other_executable(),
        )
    };
    let mut cap = Cap::empty();
    if fp.0 {
        cap |= Cap::READ;
    }
    if fp.1 {
        cap |= Cap::WRITE;
    }
    if fp.2 {
        cap |= Cap::EXECUTE;
    }
    cap
}
