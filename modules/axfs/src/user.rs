use axerrno::AxResult;
use axsync::Mutex;

static UID: Mutex<u32> = Mutex::new(2);

pub(crate) fn current_uid() -> AxResult<u32> {
    Ok(*UID.lock())
}

pub(crate) fn set_current_uid(uid: u32) -> AxResult {
    *UID.lock() = uid;
    Ok(())
}
