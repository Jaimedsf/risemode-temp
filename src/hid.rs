//! Acesso HID cru via Win32 (o mesmo caminho que SendTemp.exe: CreateFile + WriteFile).

use core::ffi::c_void;

type Handle = *mut c_void;
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

extern "system" {
    fn CreateFileW(name: *const u16, access: u32, share: u32, sa: *mut c_void, disp: u32, flags: u32, tmpl: Handle) -> Handle;
    fn WriteFile(h: Handle, buf: *const u8, n: u32, written: *mut u32, ov: *mut c_void) -> i32;
    fn CloseHandle(h: Handle) -> i32;
    fn GetLastError() -> u32;
    fn LoadLibraryW(name: *const u16) -> Handle;
    fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
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
    API.get_or_init(|| unsafe {
        let lib = |n: &str| {
            let w: Vec<u16> = n.encode_utf16().chain(Some(0)).collect();
            let h = LoadLibraryW(w.as_ptr());
            assert!(!h.is_null(), "{n} não carregou");
            h
        };
        fn sym<T>(lib: Handle, name: &[u8]) -> T {
            let p = unsafe { GetProcAddress(lib, name.as_ptr()) };
            assert!(!p.is_null(), "símbolo ausente: {}", String::from_utf8_lossy(name));
            unsafe { core::mem::transmute_copy(&p) }
        }
        let hid = lib("hid.dll");
        let cfg = lib("cfgmgr32.dll");
        Api {
            get_hid_guid: sym(hid, b"HidD_GetHidGuid\0"),
            get_attributes: sym(hid, b"HidD_GetAttributes\0"),
            get_preparsed_data: sym(hid, b"HidD_GetPreparsedData\0"),
            free_preparsed_data: sym(hid, b"HidD_FreePreparsedData\0"),
            get_caps: sym(hid, b"HidP_GetCaps\0"),
            list_size: sym(cfg, b"CM_Get_Device_Interface_List_SizeW\0"),
            list: sym(cfg, b"CM_Get_Device_Interface_ListW\0"),
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
        buf.split(|&c| c == 0)
            .filter(|s| !s.is_empty())
            .map(|s| s.iter().copied().chain(Some(0)).collect())
            .collect()
    }
}

impl Device {
    pub fn open(vid: u16, pid: u16) -> Option<Self> {
        let needle = format!("vid_{vid:04x}&pid_{pid:04x}");
        for wpath in hid_interfaces() {
            let path = String::from_utf16_lossy(&wpath[..wpath.len() - 1]);
            if !path.to_lowercase().contains(&needle) {
                continue;
            }
            unsafe {
                let h = CreateFileW(wpath.as_ptr(), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ_WRITE,
                                    core::ptr::null_mut(), OPEN_EXISTING, 0, core::ptr::null_mut());
                if h == INVALID_HANDLE_VALUE {
                    println!("CreateFile falhou em {path}: erro {}", GetLastError());
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
                return Some(Self { handle: h, path, out_len: caps.output_report_byte_length as usize });
            }
        }
        None
    }

    pub fn write(&self, report: &[u8]) -> Result<(), u32> {
        let mut written = 0u32;
        let ok = unsafe { WriteFile(self.handle, report.as_ptr(), self.out_len.min(report.len()) as u32, &mut written, core::ptr::null_mut()) };
        if ok == 0 { Err(unsafe { GetLastError() }) } else { Ok(()) }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}
