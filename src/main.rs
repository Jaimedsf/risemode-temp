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
mod win;

use std::{thread, time::Duration};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const VID: u16 = 0xAA88;
const PID: u16 = 0x8666;
const INTERVAL: Duration = Duration::from_millis(1000);
/// Peso da leitura nova na média móvel exponencial.
const SMOOTHING: f32 = 0.5;
/// Leituras seguidas com erro antes de reabrir o PawnIO (ex.: serviço reiniciado).
const REOPEN_AFTER: u32 = 5;

// Registradores SMN (de LibreHardwareMonitor Amd17Cpu.Processor.UpdateSensors).
const F17H_M01H_THM_TCON_CUR_TMP: u32 = 0x0005_9800;
const F17H_TEMP_OFFSET_FLAG: u32 = 0x0008_0000;

const USAGE: &str = "\
uso: risemode-temp.exe [opção]

  (sem opção)    roda invisível, enviando a temperatura ao display a cada segundo
  --console      roda mostrando a temperatura num console
  --once         lê a temperatura, envia uma vez e sai
  --send N       envia o valor fixo N (0 a 255) ao display; não precisa de admin
  --install      cria a tarefa agendada RiseModeTempRs (logon, como admin)
  --uninstall    fecha o processo e apaga a tarefa agendada
  --version      mostra a versão
  --help         mostra esta ajuda";

/// Converte o valor cru de THM_TCON_CUR_TMP em °C.
fn decode_tctl(raw: u32) -> f32 {
    let mut t = (raw >> 21) as f32 * 0.125;
    if raw & F17H_TEMP_OFFSET_FLAG != 0 {
        t -= 49.0;
    }
    t
}

fn read_tctl(io: &pawnio::PawnIo) -> Result<f32, String> {
    io.read_smn(F17H_M01H_THM_TCON_CUR_TMP).map(decode_tctl)
}

fn build_report(len: usize, cpu_temp: f32) -> Vec<u8> {
    let mut buf = vec![0u8; len.max(11)];
    buf[0] = 0; // report ID
    buf[1] = cpu_temp.round().clamp(0.0, 255.0) as u8;
    buf
}

/// Média móvel exponencial leve: o Tctl do Ryzen oscila muito de um segundo pro outro.
#[derive(Default)]
struct Smoother(Option<f32>);

impl Smoother {
    fn update(&mut self, t: f32) -> f32 {
        let s = match self.0 {
            Some(s) => s + (t - s) * SMOOTHING,
            None => t,
        };
        self.0 = Some(s);
        s
    }
}

#[derive(Debug, PartialEq)]
enum Mode {
    Run { console: bool, once: bool },
    Send(f32),
    Install,
    Uninstall,
    Help,
    Version,
}

fn parse_args(args: &[String]) -> Result<Mode, String> {
    let (mut console, mut once) = (false, false);
    let mut mode = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let m = match a.as_str() {
            "--console" => {
                console = true;
                continue;
            }
            "--once" => {
                once = true;
                continue;
            }
            "--install" => Mode::Install,
            "--uninstall" => Mode::Uninstall,
            "-h" | "--help" | "/?" => Mode::Help,
            "-V" | "--version" => Mode::Version,
            "--send" => {
                let v = it.next().ok_or("--send precisa de um valor (ex.: --send 88)")?;
                let t = v.parse::<f32>().ok().filter(|t| (0.0..=255.0).contains(t));
                Mode::Send(t.ok_or_else(|| format!("valor inválido para --send: {v} (use 0 a 255)"))?)
            }
            other => return Err(format!("argumento desconhecido: {other}")),
        };
        if mode.replace(m).is_some() {
            return Err("use só uma entre --send, --install, --uninstall, --help e --version".into());
        }
    }
    Ok(mode.unwrap_or(Mode::Run { console, once }))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match parse_args(&args) {
        Ok(m) => m,
        Err(e) => {
            ensure_console();
            println!("{e}\n\n{USAGE}");
            pause();
            std::process::exit(2);
        }
    };
    let code = match mode {
        Mode::Help => {
            ensure_console();
            println!("risemode-temp {VERSION}\n\n{USAGE}");
            pause();
            0
        }
        Mode::Version => {
            ensure_console();
            println!("risemode-temp {VERSION}");
            pause();
            0
        }
        Mode::Install => {
            ensure_console();
            install()
        }
        Mode::Uninstall => {
            ensure_console();
            uninstall()
        }
        Mode::Send(v) => {
            ensure_console();
            send(v)
        }
        Mode::Run { console, once } => {
            if console || once {
                ensure_console();
                println!("risemode-temp {VERSION}");
            }
            run(once)
        }
    };
    std::process::exit(code);
}

/// --send N: manda um valor fixo para o display (teste do HID, não precisa de admin).
fn send(v: f32) -> i32 {
    let code = match hid::Device::open(VID, PID) {
        Ok(d) => match d.write(&build_report(d.out_len, v)) {
            Ok(()) => {
                println!("enviado {v} °C para {} ({} bytes)", d.path, d.out_len);
                0
            }
            Err(e) => {
                println!("WriteFile falhou: erro {e}");
                1
            }
        },
        Err(e) => {
            println!("{e}");
            1
        }
    };
    pause();
    code
}

fn run(once: bool) -> i32 {
    // Instância única. O handle fica aberto até o processo sair.
    // ACCESS_DENIED: o mutex existe mas foi criado por outra sessão/elevação.
    let mutex = unsafe { win::CreateMutexW(core::ptr::null_mut(), 0, win::wide("Global\\RiseModeTempRs").as_ptr()) };
    let err = unsafe { win::GetLastError() };
    if err == win::ERROR_ALREADY_EXISTS || (mutex.is_null() && err == win::ERROR_ACCESS_DENIED) {
        println!("já está rodando");
        pause();
        return 1;
    }

    let mut io = match pawnio::PawnIo::open_amd17() {
        Ok(io) => Some(io),
        Err(e) => {
            println!("erro PawnIO: {e} (precisa rodar como administrador)");
            pause();
            return 1;
        }
    };

    let mut dev: Option<hid::Device> = None;
    let mut smooth = Smoother::default();
    let mut read_failures = 0u32;
    // Último erro logado de cada lado, para não repetir a mesma linha a cada segundo.
    let mut last_io_err = String::new();
    let mut last_hid_err = String::new();
    loop {
        if io.is_none() {
            match pawnio::PawnIo::open_amd17() {
                Ok(x) => {
                    println!("PawnIO reaberto");
                    last_io_err.clear();
                    io = Some(x);
                }
                Err(e) if e != last_io_err => {
                    println!("erro PawnIO: {e}");
                    last_io_err = e;
                }
                Err(_) => {}
            }
        }
        let temp = match io.as_ref().map(read_tctl) {
            Some(Ok(t)) => {
                read_failures = 0;
                Some(t)
            }
            Some(Err(e)) => {
                println!("falha ao ler temperatura: {e}");
                read_failures += 1;
                if read_failures >= REOPEN_AFTER {
                    println!("reabrindo o PawnIO após {read_failures} falhas seguidas");
                    io = None;
                    read_failures = 0;
                }
                None
            }
            None => None,
        };

        if let Some(temp) = temp {
            let shown = smooth.update(temp);
            if dev.is_none() {
                match hid::Device::open(VID, PID) {
                    Ok(d) => {
                        println!("display conectado: {} (report {} bytes)", d.path, d.out_len);
                        last_hid_err.clear();
                        dev = Some(d);
                    }
                    Err(e) if e != last_hid_err => {
                        println!("{e}");
                        last_hid_err = e;
                    }
                    Err(_) => {}
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
                return if sent { 0 } else { 1 };
            }
        } else if once {
            pause();
            return 1;
        }
        thread::sleep(INTERVAL);
    }
}

static OWN_CONSOLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Abre uma janela de console, a menos que a saída já esteja redirecionada (pipe/arquivo).
fn ensure_console() {
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    unsafe {
        if win::GetStdHandle(STD_OUTPUT_HANDLE).is_null() && win::AllocConsole() != 0 {
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
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
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
"#,
        exe = xml_escape(exe),
        user = xml_escape(user)
    )
}

/// schtasks lê o XML em UTF-16 LE com BOM.
fn utf16le_bom(s: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(s.encode_utf16().flat_map(|c| c.to_le_bytes()));
    bytes
}

fn install() -> i32 {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("não consegui descobrir o caminho do exe: {e}");
            pause();
            return 1;
        }
    };
    let user = format!("{}\\{}", std::env::var("USERDOMAIN").unwrap_or_default(), std::env::var("USERNAME").unwrap_or_default());
    let xml = task_xml(&exe.display().to_string(), &user);
    let xml_path = std::env::temp_dir().join("RiseModeTempRs.xml");
    if let Err(e) = std::fs::write(&xml_path, utf16le_bom(&xml)) {
        println!("não consegui gravar {}: {e}", xml_path.display());
        pause();
        return 1;
    }
    let status = std::process::Command::new("schtasks").args(["/Create", "/F", "/TN", TASK, "/XML"]).arg(&xml_path).status();
    let _ = std::fs::remove_file(&xml_path);
    let ok = matches!(status, Ok(s) if s.success());
    if ok {
        let _ = std::process::Command::new("schtasks").args(["/Run", "/TN", TASK]).status();
        println!("tarefa \"{TASK}\" criada (inicia no logon, como admin) e iniciada");
    } else {
        println!("falhou ao criar a tarefa — rode este comando num terminal como administrador");
    }
    pause();
    if ok {
        0
    } else {
        1
    }
}

fn uninstall() -> i32 {
    // Apaga a tarefa antes de matar o processo, para o RestartOnFailure não relançá-lo.
    let ok = matches!(std::process::Command::new("schtasks").args(["/Delete", "/F", "/TN", TASK]).status(), Ok(s) if s.success());
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "risemode-temp.exe", "/FI"])
        .arg(format!("PID ne {}", std::process::id()))
        .status();
    pause();
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn tctl_sem_offset() {
        // 45 °C = 360 * 0.125
        assert_eq!(decode_tctl(360 << 21), 45.0);
        assert_eq!(decode_tctl(0), 0.0);
    }

    #[test]
    fn tctl_com_offset() {
        // Faixa estendida: subtrai 49 °C.
        assert_eq!(decode_tctl((800 << 21) | F17H_TEMP_OFFSET_FLAG), 51.0);
    }

    #[test]
    fn tctl_ignora_bits_baixos() {
        assert_eq!(decode_tctl((360 << 21) | 0x7FFFF), 45.0);
    }

    #[test]
    fn report_minimo_de_11_bytes() {
        let r = build_report(0, 62.4);
        assert_eq!(r.len(), 11);
        assert_eq!(r[0], 0);
        assert_eq!(r[1], 62);
        assert!(r[2..].iter().all(|&b| b == 0));
        assert_eq!(build_report(65, 50.0).len(), 65);
    }

    #[test]
    fn report_limita_faixa() {
        assert_eq!(build_report(11, -5.0)[1], 0);
        assert_eq!(build_report(11, 300.0)[1], 255);
        assert_eq!(build_report(11, 62.5)[1], 63);
    }

    #[test]
    fn suavizacao() {
        let mut s = Smoother::default();
        assert_eq!(s.update(40.0), 40.0);
        assert_eq!(s.update(60.0), 50.0);
        assert_eq!(s.update(50.0), 50.0);
    }

    #[test]
    fn xml_escapa_caminho() {
        let x = task_xml(r"C:\A & B\<x>.exe", r"PC\joão");
        assert!(x.contains(r"<Command>C:\A &amp; B\&lt;x&gt;.exe</Command>"));
        assert!(x.contains(r"<UserId>PC\joão</UserId>"));
        assert!(x.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
    }

    #[test]
    fn xml_em_utf16_com_bom() {
        let b = utf16le_bom("A<");
        assert_eq!(b, [0xFF, 0xFE, b'A', 0, b'<', 0]);
    }

    #[test]
    fn args_padrao() {
        assert_eq!(parse_args(&[]), Ok(Mode::Run { console: false, once: false }));
        assert_eq!(parse_args(&args(&["--console"])), Ok(Mode::Run { console: true, once: false }));
        assert_eq!(parse_args(&args(&["--once"])), Ok(Mode::Run { console: false, once: true }));
    }

    #[test]
    fn args_comandos() {
        assert_eq!(parse_args(&args(&["--send", "88"])), Ok(Mode::Send(88.0)));
        assert_eq!(parse_args(&args(&["--install"])), Ok(Mode::Install));
        assert_eq!(parse_args(&args(&["--uninstall"])), Ok(Mode::Uninstall));
        assert_eq!(parse_args(&args(&["--version"])), Ok(Mode::Version));
        assert_eq!(parse_args(&args(&["/?"])), Ok(Mode::Help));
    }

    #[test]
    fn args_invalidos() {
        assert!(parse_args(&args(&["--send"])).is_err());
        assert!(parse_args(&args(&["--send", "abc"])).is_err());
        assert!(parse_args(&args(&["--send", "300"])).is_err());
        assert!(parse_args(&args(&["--send", "NaN"])).is_err());
        assert!(parse_args(&args(&["--foo"])).is_err());
        assert!(parse_args(&args(&["--install", "--uninstall"])).is_err());
    }
}
