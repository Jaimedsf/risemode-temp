//! Acesso HID cru via Win32 (o mesmo caminho que SendTemp.exe: CreateFile + WriteFile).

use crate::win::{self, CloseHandle, CreateFileW, GetLastError, Handle, WriteFile};

const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_READ_WRITE: u32 = 0x3;
const OPEN_EXISTING: u32 = 3;
const CM_GET_DEVICE_INTERFACE_LIST_PRESENT: u32 = 0;

#[repr(C)]
#[derive(Default)]
struct Guid(u32, u16, u16, [u8; 8]);

#[repr(C)]
#[derive(Default)]
struct HiddAttributes {
    size: u32,
    vendor_id: u16,
    product_id: u16,
    version: u16,
}

#[repr(C)]
#[derive(Default)]
struct HidpCaps {
    usage: u16,
    usage_page: u16,
    input_report_byte_length: u16,
    output_report_byte_length: u16,
    feature_report_byte_length: u16,
    reserved: [u16; 17],
    counts: [u16; 10],
}

// hid.dll / cfgmgr32.dll são carregadas em tempo de execução: o toolchain GNU não traz as import libs.
struct Api {
    get_hid_guid: unsafe extern "system" fn(*mut Guid),
    get_attributes: unsafe extern "system" fn(Handle, *mut HiddAttributes) -> u8,
    get_preparsed_data: unsafe extern "system" fn(Handle, *mut isize) -> u8,
    free_preparsed_data: unsafe extern "system" fn(isize) -> u8,
    get_caps: unsafe extern "system" fn(isize, *mut HidpCaps) -> i32,
    list_size: unsafe extern "system" fn(*mut u32, *const Guid, *const u16, u32) -> u32,
    list: unsafe extern "system" fn(*const Guid, *const u16, *mut u16, u32, u32) -> u32,
}

fn api() -> &'static Api {
    static API: std::sync::OnceLock<Api> = std::sync::OnceLock::new();
    API.get_or_init(|| {
        // Ambas fazem parte do Windows; se faltarem não há o que fazer além de abortar.
        let lib = |n: &str| win::load_library(n).unwrap_or_else(|| panic!("{n} não carregou"));
        fn sym<T>(lib: Handle, name: &core::ffi::CStr) -> T {
            unsafe { win::symbol(lib, name) }.unwrap_or_else(|e| panic!("{e}"))
        }
        let hid = lib("hid.dll");
        let cfg = lib("cfgmgr32.dll");
        Api {
            get_hid_guid: sym(hid, c"HidD_GetHidGuid"),
            get_attributes: sym(hid, c"HidD_GetAttributes"),
            get_preparsed_data: sym(hid, c"HidD_GetPreparsedData"),
            free_preparsed_data: sym(hid, c"HidD_FreePreparsedData"),
            get_caps: sym(hid, c"HidP_GetCaps"),
            list_size: sym(cfg, c"CM_Get_Device_Interface_List_SizeW"),
            list: sym(cfg, c"CM_Get_Device_Interface_ListW"),
        }
    })
}

pub struct Device {
    handle: Handle,
    pub path: String,
    pub out_len: usize,
}

fn hid_interfaces() -> Vec<Vec<u16>> {
    unsafe {
        let mut guid = Guid::default();
        (api().get_hid_guid)(&mut guid);
        let mut len = 0u32;
        if (api().list_size)(&mut len, &guid, core::ptr::null(), CM_GET_DEVICE_INTERFACE_LIST_PRESENT) != 0 {
            return Vec::new();
        }
        let mut buf = vec![0u16; len as usize];
        if (api().list)(&guid, core::ptr::null(), buf.as_mut_ptr(), len, CM_GET_DEVICE_INTERFACE_LIST_PRESENT) != 0 {
            return Vec::new();
        }
        buf.split(|&c| c == 0).filter(|s| !s.is_empty()).map(|s| s.iter().copied().chain(Some(0)).collect()).collect()
    }
}

impl Device {
    /// Abre a primeira interface do dispositivo que tenha relatório de saída.
    /// O erro descreve o motivo (não encontrado / CreateFile falhou), para o chamador logar.
    pub fn open(vid: u16, pid: u16) -> Result<Self, String> {
        let needle = format!("vid_{vid:04x}&pid_{pid:04x}");
        let mut err = format!("display {vid:04X}:{pid:04X} não encontrado");
        for wpath in hid_interfaces() {
            let path = String::from_utf16_lossy(&wpath[..wpath.len() - 1]);
            if !path.to_lowercase().contains(&needle) {
                continue;
            }
            unsafe {
                let h = CreateFileW(
                    wpath.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ_WRITE,
                    core::ptr::null_mut(),
                    OPEN_EXISTING,
                    0,
                    core::ptr::null_mut(),
                );
                if h == INVALID_HANDLE_VALUE {
                    err = format!("CreateFile falhou em {path}: erro {}", GetLastError());
                    continue;
                }
                let mut attr = HiddAttributes { size: core::mem::size_of::<HiddAttributes>() as u32, ..Default::default() };
                if (api().get_attributes)(h, &mut attr) == 0 || attr.vendor_id != vid || attr.product_id != pid {
                    CloseHandle(h);
                    continue;
                }
                let mut caps = HidpCaps::default();
                let mut pre = 0isize;
                if (api().get_preparsed_data)(h, &mut pre) != 0 {
                    (api().get_caps)(pre, &mut caps);
                    (api().free_preparsed_data)(pre);
                }
                if caps.output_report_byte_length == 0 {
                    CloseHandle(h);
                    continue; // interface sem relatório de saída
                }
                return Ok(Self { handle: h, path, out_len: caps.output_report_byte_length as usize });
            }
        }
        Err(err)
    }

    pub fn write(&self, report: &[u8]) -> Result<(), u32> {
        let mut written = 0u32;
        let ok =
            unsafe { WriteFile(self.handle, report.as_ptr(), self.out_len.min(report.len()) as u32, &mut written, core::ptr::null_mut()) };
        if ok == 0 {
            Err(unsafe { GetLastError() })
        } else {
            Ok(())
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}
