# Testa o executável sem hardware: só os caminhos que não precisam de PawnIO nem do display.
param([Parameter(Mandatory)][string]$Exe)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.Encoding]::UTF8   # o exe escreve UTF-8 quando a saída é um pipe
$Exe = (Resolve-Path $Exe).Path
$versao = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version

function Rodar([string[]]$ArgList, [int]$Esperado, [string]$Contem) {
    $saida = (& $Exe @ArgList | Out-String).Trim()
    $code = $LASTEXITCODE
    Write-Host "> risemode-temp $($ArgList -join ' ')  (exit $code)`n$saida`n"
    if ($code -ne $Esperado) { throw "exit $code, esperado $Esperado" }
    if ($Contem -and -not $saida.Contains($Contem)) { throw "saída não contém '$Contem'" }
}

Rodar @('--version') 0 "risemode-temp $versao"
Rodar @('--help') 0 '--install'
Rodar @('--bogus') 2 'argumento desconhecido'
Rodar @('--send', '999') 2 '0 a 255'
Write-Host "OK ($((Get-Item $Exe).Length) bytes)"
