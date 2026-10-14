//! FFI mínima com o kernel32, compartilhada pelos outros módulos.

use core::ffi::{c_void, CStr};

pub type Handle = *mut c_void;

pub const ERROR_ACCESS_DENIED: u32 = 5;
pub const ERROR_ALREADY_EXISTS: u32 = 183;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetLastError() -> u32;
    pub fn CloseHandle(h: Handle) -> i32;
    pub fn CreateFileW(name: *const u16, access: u32, share: u32, sa: *mut c_void, disp: u32, flags: u32, tmpl: Handle) -> Handle;
    pub fn WriteFile(h: Handle, buf: *const u8, n: u32, written: *mut u32, ov: *mut c_void) -> i32;
    pub fn CreateMutexW(attr: *mut c_void, owner: i32, name: *const u16) -> Handle;
    pub fn AllocConsole() -> i32;
    pub fn GetStdHandle(id: u32) -> Handle;
    fn LoadLibraryW(name: *const u16) -> Handle;
    fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
}

/// String UTF-16 terminada em zero, para as APIs `...W`.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn load_library(name: &str) -> Option<Handle> {
    let h = unsafe { LoadLibraryW(wide(name).as_ptr()) };
    (!h.is_null()).then_some(h)
}

/// Resolve um símbolo da DLL como ponteiro de função do tipo `T`.
///
/// # Safety
/// `T` precisa ser um `unsafe extern "system" fn` com a assinatura real do símbolo.
pub unsafe fn symbol<T>(lib: Handle, name: &CStr) -> Result<T, String> {
    const { assert!(core::mem::size_of::<T>() == core::mem::size_of::<*mut c_void>()) };
    let p = GetProcAddress(lib, name.as_ptr().cast());
    if p.is_null() {
        return Err(format!("símbolo {} ausente", name.to_string_lossy()));
    }
    Ok(core::mem::transmute_copy(&p))
}
