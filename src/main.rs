//! Substituto mínimo do "Rise Mode Temp CPU Driver R2.2".
//!
//! O driver original (≈ 10 MB de .NET + LibreHardwareMonitor) faz, a cada 1 s:
//!   MainForm.GetPcInfo  -> lê "Core (Tctl/Tdie)" via LibreHardwareMonitor/PawnIO
//!   MainForm.Timer_Tick -> monta 11 bytes e escreve no named pipe "tifPipe"
//!   SendTemp.exe        -> lê o pipe e escreve os bytes crus no HID VID 0xAA88 / PID 0x8666
//!
//! Layout do relatório (MainForm.btPipSnd):
//!   [0] report ID = 0   [1] CPU °C   [2] GPU °C   [3] CPU %   [4] RAM %
//!   [5] placa °C        [6] GPU %    [7..] 0
//!
//! Aqui só preenchemos [1] (temperatura da CPU), que é o que o display da bomba mostra.

#![windows_subsystem = "windows"]

mod hid;
mod pawnio;

use std::{thread, time::Duration};

const VID: u16 = 0xAA88;
const PID: u16 = 0x8666;
const INTERVAL: Duration = Duration::from_millis(1000);

// Registradores SMN (de LibreHardwareMonitor Amd17Cpu.Processor.UpdateSensors).
const F17H_M01H_THM_TCON_CUR_TMP: u32 = 0x0005_9800;
const F17H_TEMP_OFFSET_FLAG: u32 = 0x0008_0000;

fn read_tctl(io: &pawnio::PawnIo) -> Result<f32, String> {
    let raw = io.read_smn(F17H_M01H_THM_TCON_CUR_TMP)?;
    let mut t = (raw >> 21) as f32 * 0.125;
    if raw & F17H_TEMP_OFFSET_FLAG != 0 {
        t -= 49.0;
    }
    Ok(t)
}

fn build_report(len: usize, cpu_temp: f32) -> Vec<u8> {
    let mut buf = vec![0u8; len.max(11)];
    buf[0] = 0; // report ID
    buf[1] = cpu_temp.round().clamp(0.0, 255.0) as u8;
    buf
}

extern "system" {
    fn AllocConsole() -> i32;
    fn GetStdHandle(id: u32) -> *mut core::ffi::c_void;
    fn CreateMutexW(attr: *mut core::ffi::c_void, owner: i32, name: *const u16) -> *mut core::ffi::c_void;
    fn GetLastError() -> u32;
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let console = args.iter().any(|a| a == "--console" || a == "--once");
    let once = args.iter().any(|a| a == "--once");
    if args.iter().any(|a| a == "--install") {
        ensure_console();
        std::process::exit(install());
    }
    if args.iter().any(|a| a == "--uninstall") {
        ensure_console();
        std::process::exit(uninstall());
    }
    if console {
        ensure_console();
    }
    // --send N: manda um valor fixo para o display (teste do HID, não precisa de admin).
    if let Some(v) = args.iter().position(|a| a == "--send").and_then(|i| args.get(i + 1)) {
        ensure_console();
        let v: f32 = v.parse().unwrap_or(0.0);
        let code = match hid::Device::open(VID, PID) {
            Some(d) => match d.write(&build_report(d.out_len, v)) {
                Ok(()) => { println!("enviado {v} °C para {} ({} bytes)", d.path, d.out_len); 0 }
                Err(e) => { println!("WriteFile falhou: erro {e}"); 1 }
            },
            None => { println!("display {VID:04X}:{PID:04X} não encontrado"); 1 }
        };
        pause();
        std::process::exit(code);
    }

    // Instância única.
    unsafe {
        CreateMutexW(core::ptr::null_mut(), 0, wide("Global\\RiseModeTempRs").as_ptr());
        if GetLastError() == 183 {
            println!("já está rodando");
            pause();
            return;
        }
    }

    let io = match pawnio::PawnIo::open_amd17() {
        Ok(io) => io,
        Err(e) => {
            println!("erro PawnIO: {e} (precisa rodar como administrador)");
            pause();
            std::process::exit(1);
        }
    };

    let mut dev: Option<hid::Device> = None;
    let mut smooth: Option<f32> = None;
    loop {
        let temp = match read_tctl(&io) {
            Ok(t) => t,
            Err(e) => {
                println!("falha ao ler temperatura: {e}");
                thread::sleep(INTERVAL);
                continue;
            }
        };
        // Média móvel exponencial leve: o Tctl do Ryzen oscila muito de um segundo pro outro.
        let shown = match smooth {
            Some(s) => s + (temp - s) * 0.5,
            None => temp,
        };
        smooth = Some(shown);

        if dev.is_none() {
            dev = hid::Device::open(VID, PID);
            match &dev {
                Some(d) => println!("display conectado: {} (report {} bytes)", d.path, d.out_len),
                None => println!("display {VID:04X}:{PID:04X} não encontrado"),
            }
        }
        let mut sent = false;
        if let Some(d) = &dev {
            match d.write(&build_report(d.out_len, shown)) {
                Ok(()) => sent = true,
                Err(e) => {
                    println!("erro de escrita ({e}), reconectando");
                    dev = None;
                }
            }
        }
        println!("CPU Tctl {temp:5.1} °C  -> display {:3} °C {}", shown.round(), if sent { "" } else { "(não enviado)" });

        if once {
            pause();
            return;
        }
        thread::sleep(INTERVAL);
    }
}

static OWN_CONSOLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Abre uma janela de console, a menos que a saída já esteja redirecionada (pipe/arquivo).
fn ensure_console() {
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    unsafe {
        if GetStdHandle(STD_OUTPUT_HANDLE).is_null() && AllocConsole() != 0 {
            OWN_CONSOLE.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Segura a janela aberta só quando nós mesmos criamos o console.
fn pause() {
    if !OWN_CONSOLE.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    println!("Enter para sair...");
    let _ = std::io::stdin().read_line(&mut String::new());
}

const TASK: &str = "RiseModeTempRs";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Tarefa via XML: o `schtasks /SC ONLOGON` padrão mata o processo após 72 h
/// e não inicia/para na bateria; aqui fica sem limite e reinicia se cair.
fn task_xml(exe: &str, user: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Envia a temperatura da CPU para o display do water cooler Rise Mode</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>999</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
    </Exec>
  </Actions>
</Task>
"#, exe = xml_escape(exe), user = xml_escape(user))
}

fn install() -> i32 {
    let exe = std::env::current_exe().expect("caminho do exe");
    let user = format!("{}\\{}", std::env::var("USERDOMAIN").unwrap_or_default(), std::env::var("USERNAME").unwrap_or_default());
    let xml = task_xml(&exe.display().to_string(), &user);
    // schtasks lê o XML em UTF-16 LE com BOM.
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(|c| c.to_le_bytes()));
    let xml_path = std::env::temp_dir().join("RiseModeTempRs.xml");
    if let Err(e) = std::fs::write(&xml_path, bytes) {
        println!("não consegui gravar {}: {e}", xml_path.display());
        pause();
        return 1;
    }
    let status = std::process::Command::new("schtasks")
        .args(["/Create", "/F", "/TN", TASK, "/XML"])
        .arg(&xml_path)
        .status();
    let _ = std::fs::remove_file(&xml_path);
    let ok = matches!(status, Ok(s) if s.success());
    if ok {
        let _ = std::process::Command::new("schtasks").args(["/Run", "/TN", TASK]).status();
        println!("tarefa \"{TASK}\" criada (inicia no logon, como admin) e iniciada");
    } else {
        println!("falhou ao criar a tarefa — rode este comando num terminal como administrador");
    }
    pause();
    if ok { 0 } else { 1 }
}

fn uninstall() -> i32 {
    let _ = std::process::Command::new("taskkill").args(["/F", "/IM", "risemode-temp.exe", "/FI"]).arg(format!("PID ne {}", std::process::id())).status();
    let ok = matches!(std::process::Command::new("schtasks").args(["/Delete", "/F", "/TN", TASK]).status(), Ok(s) if s.success());
    pause();
    if ok { 0 } else { 1 }
}
