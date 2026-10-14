# Monta os arquivos da Release em $OutDir:
#   risemode-temp-v<versao>-windows-x64.zip   programa + scripts + docs + módulo PawnIO (LGPL)
#   risemode-temp.exe                          só o executável
#   SHA256SUMS.txt                             hashes de tudo que é distribuído
#   release-notes.md                           corpo da Release (.github/release-notes.md preenchido)
param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$Version,
    [string]$OutDir = 'dist'
)

$ErrorActionPreference = 'Stop'
$raiz = Split-Path (Split-Path $PSScriptRoot)
$Exe = (Resolve-Path $Exe).Path
Remove-Item $OutDir -Recurse -Force -ErrorAction SilentlyContinue
$stage = Join-Path $OutDir 'stage'
New-Item -ItemType Directory -Force "$stage\third_party\pawnio-modules" | Out-Null

Copy-Item $Exe $stage
Copy-Item "$raiz\scripts\*.ps1", "$raiz\README.md", "$raiz\LICENSE", "$raiz\THIRD-PARTY-NOTICES.md" $stage
Copy-Item "$raiz\third_party\pawnio-modules\*" "$stage\third_party\pawnio-modules"

$zip = Join-Path $OutDir "risemode-temp-v$Version-windows-x64.zip"
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory((Resolve-Path $stage), (Join-Path (Resolve-Path $OutDir) (Split-Path $zip -Leaf)), 'Optimal', $false)
Copy-Item $Exe $OutDir

$arquivos = @($zip, (Join-Path $OutDir 'risemode-temp.exe')) + (Get-ChildItem "$stage\*.ps1" | ForEach-Object FullName)
$somas = ($arquivos | ForEach-Object { '{0}  {1}' -f (Get-FileHash $_ -Algorithm SHA256).Hash, (Split-Path $_ -Leaf) }) -join "`n"
Set-Content (Join-Path $OutDir 'SHA256SUMS.txt') $somas -Encoding ascii -NoNewline

$notas = (Get-Content "$raiz\.github\release-notes.md" -Raw -Encoding utf8).Replace('{{VERSION}}', $Version).Replace('{{SHA256}}', $somas)
Set-Content (Join-Path $OutDir 'release-notes.md') $notas -Encoding utf8 -NoNewline
Remove-Item $stage -Recurse -Force

Write-Host $somas
