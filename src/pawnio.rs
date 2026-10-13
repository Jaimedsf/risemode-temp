//! Cliente mínimo do PawnIO (driver de kernel usado pelo LibreHardwareMonitor).
//! O PawnIO só executa módulos assinados; usamos o mesmo AMDFamily17.bin que o LHM embute.

use core::ffi::c_void;

type Handle = *mut c_void;
type FnOpen = unsafe extern "system" fn(*mut Handle) -> i32;
type FnLoad = unsafe extern "system" fn(Handle, *const u8, usize) -> i32;
type FnExecute = unsafe extern "system" fn(Handle, *const u8, *const u64, usize, *mut u64, usize, *mut usize) -> i32;
type FnClose = unsafe extern "system" fn(Handle) -> i32;

extern "system" {
    fn LoadLibraryW(name: *const u16) -> Handle;
    fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
}

static AMD_FAMILY_17: &[u8] = include_bytes!("../third_party/pawnio-modules/AMDFamily17.bin");

pub struct PawnIo {
    handle: Handle,
    execute: FnExecute,
    close: FnClose,
}

fn load_lib() -> Result<Handle, String> {
    let mut candidates = Vec::new();
    for var in ["ProgramFiles", "ProgramW6432"] {
        if let Ok(pf) = std::env::var(var) {
            candidates.push(format!("{pf}\\PawnIO\\PawnIOLib.dll"));
        }
    }
    candidates.push("PawnIOLib.dll".into());
    for c in &candidates {
        let w: Vec<u16> = c.encode_utf16().chain(Some(0)).collect();
        let h = unsafe { LoadLibraryW(w.as_ptr()) };
        if !h.is_null() {
            return Ok(h);
        }
    }
    Err("PawnIOLib.dll não encontrada (PawnIO instalado?)".into())
}

unsafe fn sym<T>(lib: Handle, name: &[u8]) -> Result<T, String> {
    let p = GetProcAddress(lib, name.as_ptr());
    if p.is_null() {
        return Err(format!("símbolo {} ausente", String::from_utf8_lossy(&name[..name.len() - 1])));
    }
    Ok(core::mem::transmute_copy(&p))
}

impl PawnIo {
    pub fn open_amd17() -> Result<Self, String> {
        check_amd_zen()?;
        let lib = load_lib()?;
        unsafe {
            let open: FnOpen = sym(lib, b"pawnio_open\0")?;
            let load: FnLoad = sym(lib, b"pawnio_load\0")?;
            let execute: FnExecute = sym(lib, b"pawnio_execute\0")?;
            let close: FnClose = sym(lib, b"pawnio_close\0")?;
            let mut handle: Handle = core::ptr::null_mut();
            let hr = open(&mut handle);
            if hr < 0 {
                return Err(format!("pawnio_open HRESULT 0x{:08X}", hr as u32));
            }
            let hr = load(handle, AMD_FAMILY_17.as_ptr(), AMD_FAMILY_17.len());
            if hr < 0 {
                close(handle);
                return Err(format!("pawnio_load HRESULT 0x{:08X}", hr as u32));
            }
            Ok(Self { handle, execute, close })
        }
    }

    /// Equivalente a LibreHardwareMonitor.PawnIo.AmdFamily17.ReadSmn.
    pub fn read_smn(&self, offset: u32) -> Result<u32, String> {
        let input = [offset as u64];
        let mut out = [0u64; 1];
        let mut ret = 0usize;
        let hr = unsafe {
            (self.execute)(self.handle, b"ioctl_read_smn\0".as_ptr(), input.as_ptr(), 1, out.as_mut_ptr(), 1, &mut ret)
        };
        if hr < 0 {
            return Err(format!("ioctl_read_smn HRESULT 0x{:08X}", hr as u32));
        }
        Ok(out[0] as u32)
    }
}

impl Drop for PawnIo {
    fn drop(&mut self) {
        unsafe { (self.close)(self.handle) };
    }
}

/// O registrador 0x59800 só vale para AMD família 17h (Zen/Zen2) e 19h (Zen3/Zen4).
fn check_amd_zen() -> Result<(), String> {
    use core::arch::x86_64::__cpuid;
    let v = __cpuid(0);
    let vendor: Vec<u8> = [v.ebx, v.edx, v.ecx].iter().flat_map(|r| r.to_le_bytes()).collect();
    if vendor != b"AuthenticAMD" {
        return Err(format!("CPU não-AMD ({})", String::from_utf8_lossy(&vendor)));
    }
    let eax = __cpuid(1).eax;
    let family = ((eax >> 8) & 0xF) + ((eax >> 20) & 0xFF);
    if !(0x17..=0x1A).contains(&family) {
        return Err(format!("família AMD 0x{family:X} não suportada"));
    }
    Ok(())
}
