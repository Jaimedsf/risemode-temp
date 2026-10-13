# Remove o risemode-temp: para o processo, apaga a tarefa agendada e a pasta em Program Files.
# Nao mexe no PawnIO (pode ser usado por outros programas, ex.: LibreHardwareMonitor).
param([switch]$SemPausa)

$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    $a = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    if ($SemPausa) { $a += ' -SemPausa' }
    Start-Process powershell.exe -Verb RunAs -ArgumentList $a
    exit
}

Start-Transcript -Path "$env:TEMP\risemode-temp-desinstalar.log" -Force | Out-Null
Set-Location $env:TEMP   # nao segurar a pasta que vai ser apagada
$destino = Join-Path $env:ProgramFiles 'RiseModeTemp'

Write-Host '1/3 Parando e removendo a tarefa agendada...'
Stop-ScheduledTask -TaskName 'RiseModeTempRs' -ErrorAction SilentlyContinue
Unregister-ScheduledTask -TaskName 'RiseModeTempRs' -Confirm:$false -ErrorAction SilentlyContinue

Write-Host '2/3 Encerrando o processo...'
Get-Process risemode-temp -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

Write-Host "3/3 Apagando $destino ..."
Remove-Item $destino -Recurse -Force -ErrorAction SilentlyContinue

if (Test-Path $destino) { Write-Host 'Nao foi possivel apagar tudo; tente de novo apos reiniciar.' -ForegroundColor Yellow }
else { Write-Host 'Desinstalado.' -ForegroundColor Green }

Stop-Transcript | Out-Null
if (-not $SemPausa) { Read-Host 'Enter para fechar' }
