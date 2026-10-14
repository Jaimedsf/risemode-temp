# risemode-temp

Substituto mínimo, escrito em Rust, do **Rise Mode Temp CPU Driver R2.2**. Ele faz só uma coisa:
mostra a temperatura da CPU no display da bomba do water cooler Rise Mode.

> **Projeto não oficial**, sem nenhuma afiliação com a Rise Mode. Use por sua conta e risco.
> Nos caminhos abaixo, `<projeto>` é a pasta onde o repositório foi clonado.

| | Original (R2.2) | risemode-temp |
|---|---|---|
| Arquivos | 22 arquivos, ~19 MB (.NET + LibreHardwareMonitor + 3 processos) | 1 arquivo `.exe`, ~395 KB |
| Processos | `Rise Mode Temp CPU Driver R2.2.exe` + `SendTemp.exe` | `risemode-temp.exe` |
| RAM | dezenas de MB (runtime .NET + interface gráfica) | ~3,4 MB em uso (working set), 6,7 MB de memória privada confirmada |
| CPU | — | ~0,007 % de um núcleo: 62 ms de CPU em 15 min rodando (Ryzen 7 5700X3D) |
| Janela | interface gráfica + ícone na bandeja | nenhuma (roda invisível) |

Os números do risemode-temp foram medidos em 14/10/2026, com a v0.2.0 instalada. A CPU é o tempo total gasto desde
que o processo iniciou: uma amostra de poucos segundos dá 0, porque o Windows conta tempo de CPU em blocos de ~15 ms.

---

## Estado atual (14/10/2026)

| Item | Situação |
|---|---|
| Programa novo | ✅ **v0.2.0** (da Release) instalado em `C:\Program Files\RiseModeTemp\risemode-temp.exe` e rodando (testado: display mostra a temperatura) |
| Início automático | ✅ tarefa agendada `RiseModeTempRs`: no logon, como administrador, sem limite de tempo, reinicia se cair |
| Driver antigo (Rise Mode R2.2) | ❌ **desinstalado**: pasta, tarefa `Tif Task` e entrada em "Programas e Recursos" apagadas |
| PawnIO 2.0.1.0 | ✅ **mantido** em `C:\Program Files\PawnIO`, serviço `PawnIO` rodando (necessário para ler o sensor) |
| Backup do instalador do PawnIO | ✅ `C:\Program Files\RiseModeTemp\PawnIO_setup.exe` |
| Rust (para recompilar) | ✅ instalado: rustup + toolchain `stable-x86_64-pc-windows-gnu` (rustc 1.99.0) |
| Código-fonte e docs | ✅ `<projeto>\` |

SHA-256 do executável instalado (v0.2.0):
`F2B9D1F4E3A0676B685A669099F6A692C82B5A6E62977450972BEEA54FA230F5`

---

## Histórico: o que foi feito

### 1. Análise do driver original

- A análise foi feita com o desmontador de IL .NET `tools\ildump.ps1`, escrito para isso. Ele usa reflexão do
  .NET Framework e **não executa nenhum código** do programa analisado.
- Os três `.exe` do driver eram .NET. Descobri:
  - `CPU Server.exe`: serviço Windows que só iniciava o `SendTemp.exe`;
  - `Rise Mode Temp CPU Driver R2.2.exe`: LibreHardwareMonitor modificado. A cada 1 s, lia `Core (Tctl/Tdie)`
    e mandava 11 bytes pelo named pipe `tifPipe`;
  - `SendTemp.exe`: recebia do pipe e escrevia no display USB (HID `VID_AA88&PID_8666`), abrindo-o em modo
    **exclusivo**;
  - inicialização pela tarefa agendada `Tif Task`, como administrador.
- No `LibreHardwareMonitorLib.dll` encontrei a leitura do sensor: o módulo PawnIO `AMDFamily17.bin`
  chamando `ioctl_read_smn(0x59800)`. Veja [Como funciona](#como-funciona).

### 2. Refeito em Rust

- Instalei o Rust: `winget install Rustlang.Rustup` e depois o toolchain GNU, porque não há Visual Studio nesta máquina.
- Escrevi `risemode-temp` (`src\main.rs`, `src\pawnio.rs`, `src\hid.rs`). Os três programas viraram um só,
  sem pipe, sem interface gráfica e sem dependências externas.
- Extraí o `AMDFamily17.bin` do `LibreHardwareMonitorLib.dll` e o embuti no `.exe`. O PawnIO só aceita
  módulos assinados, então não dava para escrever um próprio.
- Testes feitos:
  - `--send 88`: o display mostrou 88 (protocolo USB confirmado);
  - `--console` como admin: temperatura real aparecendo no display.
- Primeira instalação: tarefa `RiseModeTempRs` apontando para `target\release`, criada com `schtasks /SC ONLOGON`.

### 3. Correções após o primeiro teste

- **Limite de 72 h**: a tarefa criada pelo `schtasks` padrão fecharia o programa depois de 3 dias ligado
  (e não rodaria na bateria). O `--install` passou a gerar a tarefa por XML: sem limite de tempo, reinício
  automático em 1 min se cair, e só uma instância.
- **Local de instalação**: o `.exe` saiu de `<projeto>\target\release` (pasta do usuário,
  apagada por um `cargo clean`) para `C:\Program Files\RiseModeTemp`. Como roda como administrador, precisa
  ficar onde só um admin pode alterá-lo.
- Criei os scripts `instalar.ps1`, `desinstalar.ps1` e `remover-antigo.ps1`, que pedem administrador sozinhos.

### 4. Migração (13/10/2026, 17:06)

1. `instalar.ps1`: copiou o programa para `C:\Program Files\RiseModeTemp`, recriou a tarefa `RiseModeTempRs`
   pelo XML e iniciou o programa.
2. `remover-antigo.ps1`: guardou o `PawnIO_setup.exe`, rodou o desinstalador oficial em modo silencioso e
   limpou as sobras.
3. Conferi tudo depois. Resultado: o [estado atual](#estado-atual-14102026) acima.

---

## O que foi removido

**Programa "Rise Mode Temp CPU Driver R2.2" inteiro**, pelo desinstalador oficial + limpeza:

- Pasta `C:\Program Files (x86)\Rise Mode Temp CPU Driver R2.2\` (22 arquivos, ~19 MB):

  | Arquivo | O que era |
  |---|---|
  | `Rise Mode Temp CPU Driver R2.2.exe` (7,2 MB) | programa principal (interface + leitura de sensores) |
  | `SendTemp.exe` | ponte pipe → display USB |
  | `CPU Server.exe` | serviço que iniciava o SendTemp |
  | `LibreHardwareMonitorLib.dll` / `.xml` / `.config` | biblioteca de sensores |
  | `HidSharp.dll`, `RAMSPDToolkit-NDD.dll` | USB/HID e leitura de SPD da RAM |
  | `Aga.Controls.dll` / `.pdb`, `OxyPlot.dll`, `OxyPlot.WindowsForms.dll` | interface gráfica e gráficos |
  | `Newtonsoft.Json.dll`, `System.CodeDom.dll`, `Microsoft.Win32.TaskScheduler.dll` | dependências .NET |
  | `PawnIO_setup.exe` (5 MB) | instalador do PawnIO (**cópia guardada** em `C:\Program Files\RiseModeTemp`) |
  | `cpu4.ico`, `Cyberiada_Medium.otf`, `License.html`, `ini.txt` | ícone, fonte, licença, config vazia |
  | `unins000.exe` / `unins000.dat` | desinstalador |

- Tarefa agendada **`Tif Task`**: iniciava o driver antigo no logon. Antes de apagar, desativei.
- Serviço **`tifSer`** (`CPU Server.exe`): o desinstalador o para e apaga. Não está mais registrado.
- Entrada **"Rise Mode Temp CPU Driver R2.2 version 2.2"** em Configurações → Aplicativos.
- Processos `Rise Mode Temp CPU Driver R2.2.exe` e `SendTemp.exe`, encerrados.

**Restos temporários do desenvolvimento:**

- a primeira versão da tarefa `RiseModeTempRs`, que apontava para `target\release` e tinha o limite de 72 h. Foi substituída;
- `target\release\risemode-temp.old.exe`, a cópia renomeada durante a recompilação.

**O que NÃO foi removido, de propósito:**

- **PawnIO** (`C:\Program Files\PawnIO`, serviço `PawnIO`): o programa novo precisa dele.

### Por que o PawnIO não foi reescrito

Ler o sensor do Ryzen (registrador SMN) exige código rodando no **kernel**. Avaliado em 13/10/2026:

- **Driver próprio em Rust**: é possível escrever (`windows-drivers-rs`), mas o Windows 11 só carrega driver
  de kernel **assinado pela Microsoft**. O `PawnIO.sys` é assinado por *Microsoft Windows Hardware
  Compatibility Publisher*. Para ter a mesma assinatura:
  - certificado EV pago todo ano;
  - conta no Partner Center;
  - envio do driver para a Microsoft.

  Sem isso, só funciona em *test signing*, que tem estes custos:
  - exige desligar o Secure Boot;
  - deixa marca d'água na tela;
  - faz anti-cheats (Vanguard, FACEIT etc.) recusarem rodar;
  - reduz a segurança do sistema.

  Além disso, um bug em driver de kernel causa tela azul.
- **Módulo PawnIO próprio**: o PawnIO só executa módulos assinados pelo autor dele. Por isso reaproveitamos o `AMDFamily17.bin`.
- **Sem driver**: testado. Esta placa não expõe temperatura pelo Windows: os contadores
  `Thermal Zone Information` não existem e a classe `MSAcpi_ThermalZoneTemperature` responde "Sem suporte".
- **WinRing0** (driver antigo): está na lista de drivers vulneráveis bloqueados da Microsoft. Seria pior.

O que **daria** para refazer é só a parte de usuário, a `PawnIOLib.dll` (23 KB): falar direto com o driver
via `DeviceIoControl`. O ganho seria eliminar uma DLL, mas o driver `PawnIO.sys` continuaria necessário.

O PawnIO é código aberto (namazso/PawnIO), assinado pela Microsoft, e é o mesmo que o LibreHardwareMonitor usa.

## O que foi adicionado ao sistema

| Item | Onde | Tamanho | Como remover |
|---|---|---|---|
| risemode-temp | `C:\Program Files\RiseModeTemp\` | ~5,6 MB (5 MB são o backup do PawnIO) | `desinstalar.ps1` |
| Tarefa `RiseModeTempRs` | Agendador de Tarefas | — | `desinstalar.ps1` |
| Código-fonte | `<projeto>\` | ~1 MB | apagar a pasta |
| Rust (rustup + toolchain GNU) | `%USERPROFILE%\.rustup`, `%USERPROFILE%\.cargo` | ~2,3 GB | `rustup self uninstall` (só se não for mais recompilar) |

### Como voltar ao driver original

1. Rode `desinstalar.ps1`.
2. Reinstale o "Rise Mode Temp CPU Driver" pelo instalador do site da Rise Mode.

O PawnIO já instalado é reaproveitado.

---

## Onde ficam os arquivos

| O quê | Caminho |
|---|---|
| **Executável instalado** | `C:\Program Files\RiseModeTemp\risemode-temp.exe` |
| Esta documentação (cópia) | `C:\Program Files\RiseModeTemp\LEIAME.md` |
| Desinstalador | `C:\Program Files\RiseModeTemp\desinstalar.ps1` |
| Instalador do PawnIO (backup) | `C:\Program Files\RiseModeTemp\PawnIO_setup.exe` |
| Código-fonte | `<projeto>\` |
| Executável compilado | `<projeto>\target\release\risemode-temp.exe` |
| Scripts | `<projeto>\scripts\` |
| Tarefa de inicialização | Agendador de Tarefas → `RiseModeTempRs` |
| Logs dos scripts | `%TEMP%\risemode-temp-*.log` |

---

## Requisitos

- **CPU AMD Ryzen** (família 17h/19h/1Ah: Zen, Zen 2, Zen 3, Zen 4, Zen 5). Em CPU Intel o programa se recusa a rodar.
- **PawnIO** instalado (`C:\Program Files\PawnIO`). É o driver de kernel que permite ler o sensor da CPU.
  Ele veio junto com o driver original e **não** é removido com ele. Se sumir, reinstale com
  `C:\Program Files\RiseModeTemp\PawnIO_setup.exe` ou baixe em <https://pawnio.eu>.
- **Rodar como administrador**, porque o PawnIO exige. A tarefa agendada já cuida disso.
- Display do water cooler conectado por USB. Ele aparece como dispositivo HID `VID_AA88&PID_8666`.

---

## Instalar

### Pela Release (sem precisar do Rust)

1. Baixe o `risemode-temp-vX.Y.Z-windows-x64.zip` em
   [Releases](https://github.com/Jaimedsf/risemode-temp/releases) e extraia numa pasta qualquer.
2. **Desbloqueie os arquivos.** O Windows marca o que veio da internet e se recusa a rodar os `.ps1`. Clique com o
   botão direito no `.zip` **antes de extrair** → **Propriedades** → marque **Desbloquear** → OK. Ou, depois de
   extrair, rode num PowerShell dentro da pasta: `Get-ChildItem | Unblock-File`.
3. Clique com o botão direito em **`instalar.ps1`** → **Executar com o PowerShell** e aceite o UAC.

O `.exe` não é assinado digitalmente, então o SmartScreen pode avisar "O Windows protegeu o computador". O SHA-256
de cada arquivo está nas notas da Release, para você conferir com `Get-FileHash risemode-temp.exe`.

### A partir do código-fonte

1. Compile (veja [Recompilar](#recompilar)) e abra a pasta `<projeto>\scripts`.
2. Clique com o botão direito em **`instalar.ps1`** → **Executar com o PowerShell**.
3. Aceite o pedido de administrador (UAC).

O script:
1. para uma versão que já esteja rodando;
2. copia o `.exe` para `C:\Program Files\RiseModeTemp`;
3. desativa a tarefa `Tif Task` do driver antigo e fecha os processos dele;
4. cria a tarefa agendada `RiseModeTempRs` e já inicia o programa.

Pelo terminal, em um PowerShell **como administrador**:

```powershell
powershell -ExecutionPolicy Bypass -File <projeto>\scripts\instalar.ps1
```

## Desinstalar

- Clique com o botão direito em `C:\Program Files\RiseModeTemp\desinstalar.ps1` → **Executar com o PowerShell**,
  ou use o mesmo arquivo em `<projeto>\scripts\`.

O script para o processo, apaga a tarefa `RiseModeTempRs` e remove `C:\Program Files\RiseModeTemp`.
O PawnIO **continua** instalado. Para removê-lo também: Configurações → Aplicativos → **PawnIO** → Desinstalar.

## Remover o driver antigo (Rise Mode R2.2)

- Clique com o botão direito em `scripts\remover-antigo.ps1` → **Executar com o PowerShell**.

O script:
1. fecha `Rise Mode Temp CPU Driver R2.2.exe` e `SendTemp.exe`;
2. guarda uma cópia do `PawnIO_setup.exe` em `C:\Program Files\RiseModeTemp`;
3. roda o desinstalador oficial em modo silencioso;
4. apaga a `Tif Task` e a pasta `C:\Program Files (x86)\Rise Mode Temp CPU Driver R2.2`;
5. confere se o PawnIO continua instalado.

> Rode o `instalar.ps1` **antes**, para que a cópia do instalador do PawnIO tenha onde ser guardada.

---

## Iniciar junto com o Windows

O `instalar.ps1` já deixa isso configurado. Ele cria a tarefa agendada **`RiseModeTempRs`**, que:

- dispara **no logon** do seu usuário;
- roda **com privilégios mais altos** (administrador), sem pedir UAC a cada boot;
- **não tem limite de tempo**. A tarefa criada por `schtasks /SC ONLOGON` pararia o programa depois de 72 h;
- **reinicia sozinha** 1 minuto depois, se o processo cair;
- roda também na bateria (relevante só em notebook).

Comandos úteis, no PowerShell **como administrador**:

```powershell
Start-ScheduledTask   -TaskName RiseModeTempRs   # iniciar agora
Stop-ScheduledTask    -TaskName RiseModeTempRs   # parar
Disable-ScheduledTask -TaskName RiseModeTempRs   # não iniciar mais no logon
Enable-ScheduledTask  -TaskName RiseModeTempRs   # voltar a iniciar no logon
Get-ScheduledTask     -TaskName RiseModeTempRs | Select TaskName, State
```

Pela interface gráfica: Iniciar → **Agendador de Tarefas** → Biblioteca → `RiseModeTempRs`.

O próprio executável também cria e remove a tarefa:

```powershell
& "C:\Program Files\RiseModeTemp\risemode-temp.exe" --install     # cria a tarefa (aponta para onde o exe está)
& "C:\Program Files\RiseModeTemp\risemode-temp.exe" --uninstall   # fecha o processo e apaga a tarefa
```

---

## Ver quanto está consumindo

- **Gerenciador de Tarefas** (Ctrl+Shift+Esc) → aba **Detalhes** → `risemode-temp.exe`.

Pelo PowerShell:

```powershell
$p = Get-Process risemode-temp
"RAM: {0:N1} MB em uso (privada {1:N1} MB)" -f ($p.WorkingSet64/1MB), ($p.PrivateMemorySize64/1MB)
$seg = ((Get-Date) - $p.StartTime).TotalSeconds
"CPU: {0:N0} ms em {1:N0} min = {2:N3} % de um núcleo" -f $p.TotalProcessorTime.TotalMilliseconds, ($seg/60), ($p.TotalProcessorTime.TotalMilliseconds / $seg / 10)
```

A CPU é calculada sobre todo o tempo desde que o processo iniciou. Medir só alguns segundos costuma dar 0, porque o
Windows conta tempo de CPU em blocos de ~15 ms e o programa gasta menos que isso.

A cada segundo, o programa faz uma leitura do sensor (1 chamada ao PawnIO) e uma escrita USB. No resto do tempo fica dormindo.

---

## Modos de uso (diagnóstico)

O programa roda sem janela. Para ver o que está acontecendo, primeiro **pare a tarefa**
(só pode existir uma instância) e rode num PowerShell **como administrador**:

| Comando | O que faz |
|---|---|
| `risemode-temp.exe --console` | roda normalmente, mostrando a temperatura a cada segundo |
| `risemode-temp.exe --once` | lê a temperatura, envia uma vez ao display e sai |
| `risemode-temp.exe --send 88` | manda o valor fixo `88` ao display. Testa o USB e **não precisa de admin** |
| `risemode-temp.exe --install` / `--uninstall` | cria/remove a tarefa agendada |
| `risemode-temp.exe --version` / `--help` | mostra a versão / todas as opções |

### Problemas comuns

| Mensagem | Causa / solução |
|---|---|
| `erro PawnIO: ... 0x80070005` | não está rodando como administrador |
| `PawnIOLib.dll não encontrada` | PawnIO não instalado. Rode `PawnIO_setup.exe` |
| `CreateFile falhou ... erro 32` | outro programa está com o display aberto, geralmente o driver antigo (`SendTemp.exe`). Feche-o ou rode `remover-antigo.ps1` |
| `display AA88:8666 não encontrado` | cabo USB do display desconectado. O programa tenta de novo a cada segundo |
| `já está rodando` | já existe uma instância (a da tarefa). Pare a tarefa antes de testar |
| antivírus apaga ou bloqueia o `.exe` (ex.: Kaspersky `VHO:Trojan-Downloader.Win32.Remus.gen`) | falso positivo da análise heurística, comum em executáveis pequenos e sem assinatura. Confira o SHA-256 e o atestado da Release (veja abaixo), adicione uma exceção e, se quiser, reporte o falso positivo ao fabricante do antivírus |

---

## Como funciona

Descobri o funcionamento por engenharia reversa do IL .NET do driver original:

- `CPU Server.exe`: serviço que só inicia o `SendTemp.exe`.
- `Rise Mode Temp CPU Driver R2.2.exe`: cópia do LibreHardwareMonitor. A cada 1 s, lê o sensor
  `Core (Tctl/Tdie)` e escreve 11 bytes no named pipe `tifPipe`.
- `SendTemp.exe`: lê o pipe e escreve os bytes crus (`WriteFile`) no HID `VID 0xAA88 / PID 0x8666`.

Formato do relatório enviado ao display:

| Byte | Conteúdo | risemode-temp envia |
|---|---|---|
| 0 | report ID (`0`) | `0` |
| 1 | temperatura da CPU (°C) | ✅ |
| 2 | temperatura da GPU (°C) | 0 |
| 3 | uso da CPU (%) | 0 |
| 4 | uso da RAM (%) | 0 |
| 5 | temperatura da placa-mãe (°C) | 0 |
| 6 | uso da GPU (%) | 0 |
| 7… | zeros | 0 |

Leitura da temperatura, igual ao LibreHardwareMonitor `Amd17Cpu`:
pelo PawnIO, carrega o módulo assinado `AMDFamily17.bin` (extraído do `LibreHardwareMonitorLib.dll`
e embutido no `.exe`) e chama `ioctl_read_smn(0x59800)`:

```
temp = (raw >> 21) * 0.125
se (raw & 0x80000) != 0: temp -= 49
```

O valor passa por uma média móvel leve (α = 0,5), porque o Tctl do Ryzen oscila muito de um segundo para o outro.

### Estrutura do código

```
src/main.rs       laço principal, argumentos, criação da tarefa agendada (XML)
src/pawnio.rs     cliente do PawnIO (carrega PawnIOLib.dll dinamicamente) + checagem de CPU
src/hid.rs        enumeração (cfgmgr32) e escrita no HID (hid.dll + WriteFile)
src/win.rs        FFI mínima com o kernel32, compartilhada pelos módulos
.github/          CI, Release automática e scripts de smoke test/empacotamento
third_party/pawnio-modules/  módulo PawnIO AMDFamily17 (.bin assinado + fonte .p, LGPL-2.1)
scripts/          instalar.ps1, desinstalar.ps1, remover-antigo.ps1
tools/ildump.ps1  desmontador de IL .NET usado na análise do driver original
```

Para refazer a análise: `powershell -File tools\ildump.ps1 -Path "<arquivo .exe/.dll .NET>" > saida.txt`.
Use `-ListOnly` para listar só classes e métodos, ou `-TypeFilter 'Regex'` para filtrar classes.

Não usa nenhuma crate externa, só FFI direto com a API do Windows.

## Recompilar

Requisitos: Rust (`winget install Rustlang.Rustup`) com o toolchain GNU. Não há MSVC nesta máquina.

```powershell
rustup default stable-x86_64-pc-windows-gnu
git clone https://github.com/Jaimedsf/risemode-temp
cd risemode-temp
cargo build --release
# depois rode scripts\instalar.ps1 para atualizar a cópia em Program Files
```

> Se a versão antiga estiver rodando, o `cargo build` pode falhar com "Acesso negado" ao sobrescrever
> o `.exe`. Pare a tarefa antes, ou renomeie o `.exe` antigo.

Antes de mandar mudanças, rode o mesmo que o CI:

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

> `opt-level = "s"` (e não `"z"`) no `Cargo.toml` é de propósito: em 13/10/2026 o build com `"z"` era apagado pelo
> Kaspersky como falso positivo (`VHO:Trojan-Downloader.Win32.Remus.gen`), e o com `"s"` não, com só ~1 KB a mais.

## CI e Releases

- **CI** (`.github/workflows/ci.yml`): a cada push e pull request, no Windows, com os toolchains
  `x86_64-pc-windows-gnu` e `x86_64-pc-windows-msvc`: formatação, clippy, testes, build release e um smoke test do
  `.exe` (`--version`, `--help` e argumentos inválidos). O `.exe` de cada toolchain fica disponível como artefato.
- **Release** (`.github/workflows/release.yml`): publica sozinha quando a versão do `Cargo.toml` ainda não tem tag.
  O `.exe` publicado é o do toolchain GNU, o mesmo usado para desenvolver e testar com o hardware.

Para lançar uma versão (a `main` é protegida, então a versão muda sempre por PR):

1. **Pelo GitHub:** Actions → **Preparar release** → **Run workflow** → escolha `patch`, `minor` ou `major`. O
   workflow abre o PR `release: vX.Y.Z` e roda o CI nele. Quando ficar verde, faça o merge: o workflow **Release**
   compila, testa, cria a tag `vX.Y.Z` e publica.
2. **Local:** numa branch, mude `version` no `Cargo.toml`, rode `cargo build` (atualiza o `Cargo.lock`), faça commit
   dos dois, abra um PR e faça o merge depois do CI.

Uma versão com sufixo (ex.: `0.3.0-rc.1`) sai como *pre-release*. A Release leva o `.zip`, o `.exe`, o
`SHA256SUMS.txt` e um [atestado de proveniência](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations)
que prova que o `.exe` foi compilado pelo GitHub Actions a partir deste repositório:

```powershell
gh attestation verify risemode-temp.exe --repo Jaimedsf/risemode-temp
```

### Proteção da `main`

A `main` tem um *ruleset* (Settings → Rules → Rulesets → `Proteger main`) que:

- bloqueia push direto: toda mudança entra por pull request;
- exige o CI verde nos dois toolchains (`stable-x86_64-pc-windows-gnu` e `stable-x86_64-pc-windows-msvc`);
- bloqueia force push e a exclusão da branch.

Como o projeto tem um mantenedor só, o PR não exige aprovação. O fluxo do dia a dia:

```powershell
git switch -c minha-mudanca
# ... editar, commitar ...
git push -u origin minha-mudanca
gh pr create --fill
gh pr merge --squash --delete-branch   # depois que o CI passar (ou use --auto para esperar sozinho)
```

---

## Licença

- Código deste projeto (`src/`, `scripts/`, `tools/`, documentação): **MIT**. Veja [`LICENSE`](LICENSE).
- `third_party/pawnio-modules/` (módulo `AMDFamily17`, de namazso): **LGPL-2.1-or-later**. O código-fonte e o texto
  da licença acompanham o binário. Veja [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md).
- O driver PawnIO **não** é distribuído aqui. Instale-o pelo site oficial: <https://pawnio.eu>.
- Nenhum código ou binário da Rise Mode é distribuído. "Rise Mode" aparece só para indicar compatibilidade.
