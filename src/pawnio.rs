//! Cliente mínimo do PawnIO (driver de kernel usado pelo LibreHardwareMonitor).
//! O PawnIO só executa módulos assinados; usamos o mesmo AMDFamily17.bin que o LHM embute.

use crate::win::{self, Handle};

type FnOpen = unsafe extern "system" fn(*mut Handle) -> i32;
type FnLoad = unsafe extern "system" fn(Handle, *const u8, usize) -> i32;
type FnExecute = unsafe extern "system" fn(Handle, *const u8, *const u64, usize, *mut u64, usize, *mut usize) -> i32;
type FnClose = unsafe extern "system" fn(Handle) -> i32;

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
    candidates.iter().find_map(|c| win::load_library(c)).ok_or_else(|| "PawnIOLib.dll não encontrada (PawnIO instalado?)".into())
}

fn hresult(op: &str, hr: i32) -> String {
    format!("{op} HRESULT 0x{:08X}", hr as u32)
}

impl PawnIo {
    pub fn open_amd17() -> Result<Self, String> {
        check_amd_zen()?;
        let lib = load_lib()?;
        unsafe {
            let open: FnOpen = win::symbol(lib, c"pawnio_open")?;
            let load: FnLoad = win::symbol(lib, c"pawnio_load")?;
            let execute: FnExecute = win::symbol(lib, c"pawnio_execute")?;
            let close: FnClose = win::symbol(lib, c"pawnio_close")?;
            let mut handle: Handle = core::ptr::null_mut();
            let hr = open(&mut handle);
            if hr < 0 {
                return Err(hresult("pawnio_open", hr));
            }
            let hr = load(handle, AMD_FAMILY_17.as_ptr(), AMD_FAMILY_17.len());
            if hr < 0 {
                close(handle);
                return Err(hresult("pawnio_load", hr));
            }
            Ok(Self { handle, execute, close })
        }
    }

    /// Equivalente a LibreHardwareMonitor.PawnIo.AmdFamily17.ReadSmn.
    pub fn read_smn(&self, offset: u32) -> Result<u32, String> {
        let input = [offset as u64];
        let mut out = [0u64; 1];
        let mut ret = 0usize;
        let hr =
            unsafe { (self.execute)(self.handle, c"ioctl_read_smn".as_ptr().cast(), input.as_ptr(), 1, out.as_mut_ptr(), 1, &mut ret) };
        if hr < 0 {
            return Err(hresult("ioctl_read_smn", hr));
        }
        Ok(out[0] as u32)
    }
}

impl Drop for PawnIo {
    fn drop(&mut self) {
        unsafe { (self.close)(self.handle) };
    }
}

/// Família "exibida" a partir do EAX do CPUID 1: a família estendida só soma quando a base é 0xF.
fn cpu_family(eax: u32) -> u32 {
    let base = (eax >> 8) & 0xF;
    if base == 0xF {
        base + ((eax >> 20) & 0xFF)
    } else {
        base
    }
}

/// O registrador 0x59800 só vale para AMD família 17h (Zen/Zen2), 19h (Zen3/Zen4) e 1Ah (Zen5).
fn check_amd_zen() -> Result<(), String> {
    use core::arch::x86_64::__cpuid;
    let v = __cpuid(0);
    let vendor: Vec<u8> = [v.ebx, v.edx, v.ecx].iter().flat_map(|r| r.to_le_bytes()).collect();
    if vendor != b"AuthenticAMD" {
        return Err(format!("CPU não-AMD ({})", String::from_utf8_lossy(&vendor)));
    }
    let family = cpu_family(__cpuid(1).eax);
    if !(0x17..=0x1A).contains(&family) {
        return Err(format!("família AMD 0x{family:X} não suportada"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn familia_zen() {
        assert_eq!(cpu_family(0x00A2_0F12), 0x19); // Ryzen 7 5700X3D (Zen 3)
        assert_eq!(cpu_family(0x0087_0F10), 0x17); // Ryzen 3000 (Zen 2)
        assert_eq!(cpu_family(0x00B4_0F40), 0x1A); // Ryzen 9000 (Zen 5)
    }

    #[test]
    fn familia_estendida_so_com_base_f() {
        // Base 6 com bits de família estendida setados: a estendida deve ser ignorada.
        assert_eq!(cpu_family(0x0010_0600), 0x6);
    }

    #[test]
    fn modulo_embutido_nao_vazio() {
        assert!(AMD_FAMILY_17.len() > 1024);
    }
}
