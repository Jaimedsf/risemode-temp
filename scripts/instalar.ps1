# Instala o risemode-temp em C:\Program Files\RiseModeTemp e agenda para iniciar no logon.
# Uso: clique direito > "Executar com o PowerShell" (pede administrador sozinho).
param([switch]$SemPausa)

$ErrorActionPreference = 'Stop'
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    $a = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    if ($SemPausa) { $a += ' -SemPausa' }
    Start-Process powershell.exe -Verb RunAs -ArgumentList $a
    exit
}

Start-Transcript -Path "$env:TEMP\risemode-temp-instalar.log" -Force | Out-Null
$destino = Join-Path $env:ProgramFiles 'RiseModeTemp'
$projeto = Split-Path $PSScriptRoot

# Procura o exe: ao lado do script (pacote) ou no build do projeto.
$exe = @("$PSScriptRoot\risemode-temp.exe", "$projeto\target\release\risemode-temp.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $exe) { throw 'risemode-temp.exe nao encontrado (rode "cargo build --release" no projeto).' }

Write-Host '1/4 Parando versao em execucao (se houver)...'
Stop-ScheduledTask -TaskName 'RiseModeTempRs' -ErrorAction SilentlyContinue
Get-Process risemode-temp -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

Write-Host "2/4 Copiando para $destino ..."
New-Item -ItemType Directory -Force $destino | Out-Null
if ((Resolve-Path $exe).Path -ne (Join-Path $destino 'risemode-temp.exe')) {
    Copy-Item $exe $destino -Force
}
foreach ($f in "$projeto\README.md", "$PSScriptRoot\README.md") {
    if (Test-Path $f) { Copy-Item $f (Join-Path $destino 'LEIAME.md') -Force; break }
}
Copy-Item "$PSScriptRoot\desinstalar.ps1" $destino -Force -ErrorAction SilentlyContinue

Write-Host '3/4 Desativando a tarefa do driver antigo (se existir)...'
Disable-ScheduledTask -TaskName 'Tif Task' -ErrorAction SilentlyContinue | Out-Null
Get-Process 'Rise Mode Temp CPU Driver R2.2', 'SendTemp' -ErrorAction SilentlyContinue | Stop-Process -Force

Write-Host '4/4 Criando tarefa agendada e iniciando...'
& (Join-Path $destino 'risemode-temp.exe') --install | Out-Host
Start-Sleep -Seconds 2
$p = Get-Process risemode-temp -ErrorAction SilentlyContinue
if ($p) { Write-Host "OK: rodando (PID $($p.Id)) a partir de $destino" -ForegroundColor Green }
else { Write-Host 'ATENCAO: o processo nao iniciou. Teste com: risemode-temp.exe --console' -ForegroundColor Yellow }

Stop-Transcript | Out-Null
if (-not $SemPausa) { Read-Host 'Enter para fechar' }
