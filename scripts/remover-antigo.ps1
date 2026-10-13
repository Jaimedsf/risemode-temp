# Remove o "Rise Mode Temp CPU Driver R2.2" original, mantendo o PawnIO instalado.
# O desinstalador original (Inno Setup) apaga a "Tif Task" e o servico tifSer; nao toca no PawnIO.
param([switch]$SemPausa)

$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    $a = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    if ($SemPausa) { $a += ' -SemPausa' }
    Start-Process powershell.exe -Verb RunAs -ArgumentList $a
    exit
}

Start-Transcript -Path "$env:TEMP\risemode-temp-remover-antigo.log" -Force | Out-Null
Set-Location $env:TEMP
$antigo = 'C:\Program Files (x86)\Rise Mode Temp CPU Driver R2.2'
$novo = Join-Path $env:ProgramFiles 'RiseModeTemp'

Write-Host '1/4 Encerrando processos antigos...'
Get-Process 'Rise Mode Temp CPU Driver R2.2', 'SendTemp', 'CPU Server' -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# Guarda o instalador do PawnIO, caso um dia precise reinstalar o driver.
if ((Test-Path "$antigo\PawnIO_setup.exe") -and (Test-Path $novo)) {
    Copy-Item "$antigo\PawnIO_setup.exe" $novo -Force
    Write-Host "    copia de seguranca: $novo\PawnIO_setup.exe"
}

Write-Host '2/4 Rodando o desinstalador original...'
if (Test-Path "$antigo\unins000.exe") {
    Start-Process "$antigo\unins000.exe" -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait
    Start-Sleep -Seconds 2
} else {
    Write-Host '    (desinstalador nao encontrado, seguindo com a limpeza manual)'
}

Write-Host '3/4 Limpando sobras...'
Unregister-ScheduledTask -TaskName 'Tif Task' -Confirm:$false -ErrorAction SilentlyContinue
if (Get-Service tifSer -ErrorAction SilentlyContinue) { sc.exe stop tifSer | Out-Null; sc.exe delete tifSer | Out-Null }
Remove-Item $antigo -Recurse -Force -ErrorAction SilentlyContinue

Write-Host '4/4 Verificando...'
$ok = $true
if (Test-Path $antigo) { Write-Host "    pasta antiga ainda existe: $antigo" -ForegroundColor Yellow; $ok = $false }
if (Get-ScheduledTask -TaskName 'Tif Task' -ErrorAction SilentlyContinue) { Write-Host '    "Tif Task" ainda existe' -ForegroundColor Yellow; $ok = $false }
if (Test-Path "$env:ProgramFiles\PawnIO\PawnIOLib.dll") { Write-Host '    PawnIO continua instalado (necessario).' -ForegroundColor Green }
else { Write-Host '    PawnIO NAO encontrado! Reinstale com PawnIO_setup.exe.' -ForegroundColor Red; $ok = $false }
if ($ok) { Write-Host 'Driver antigo removido.' -ForegroundColor Green }

Stop-Transcript | Out-Null
if (-not $SemPausa) { Read-Host 'Enter para fechar' }
