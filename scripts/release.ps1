#Requires -Version 5.1
<#
.SYNOPSIS
    Compila, firma y publica una nueva release de Lufal Ordenes en GitHub.
.DESCRIPTION
    Prerequisitos (una sola vez):
      - winget install GitHub.cli
      - gh auth login
      - Clave privada en C:\Users\<tu-usuario>\.tauri\lufal-ordenes.key
    Uso:
      .\scripts\release.ps1
#>

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent

function Write-Step { param($msg) Write-Host "`n  ► $msg" -ForegroundColor Cyan }
function Write-Ok   { param($msg) Write-Host "    ✔ $msg" -ForegroundColor Green }
function Write-Fail { param($msg) Write-Host "`n  ✘ ERROR: $msg`n" -ForegroundColor Red; exit 1 }

Write-Host "`n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host "  Lufal Ordenes — Script de Release" -ForegroundColor White
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`n" -ForegroundColor DarkGray

# ── Verificar prerequisitos ──────────────────────────────────────────────────
Write-Step "Verificando prerequisitos..."
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    Write-Fail "gh CLI no está instalado. Ejecuta: winget install GitHub.cli"
}
Write-Ok "gh CLI encontrado"

# ── Leer versión ─────────────────────────────────────────────────────────────
Write-Step "Leyendo versión del proyecto..."
$tauriConf = Get-Content "$projectRoot\src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$version   = $tauriConf.version
$tag       = "v$version"
Write-Ok "Versión: $version  →  tag: $tag"

# ── Verificar que el tag no exista ───────────────────────────────────────────
$existingTag = git -C $projectRoot tag -l $tag 2>&1
if ($existingTag -eq $tag) {
    Write-Fail "El tag '$tag' ya existe en el repo. Sube la versión en src-tauri/tauri.conf.json y package.json antes de continuar."
}

# ── Confirmar ────────────────────────────────────────────────────────────────
$confirm = Read-Host "  ¿Publicar release $tag? (s/N)"
if ($confirm -notin @('s', 'S')) { Write-Host "`n  Cancelado.`n"; exit 0 }

# ── Leer clave de firma ──────────────────────────────────────────────────────
Write-Step "Cargando clave de firma..."
$keyPath = "$env:USERPROFILE\.tauri\lufal-ordenes.key"
if (-not (Test-Path $keyPath)) {
    Write-Fail "No se encontró la clave en $keyPath`n  Asegúrate de que el archivo existe."
}
$privateKey = (Get-Content $keyPath -Raw).Trim()
Write-Ok "Clave cargada desde $keyPath"

# ── Leer password (de .env o pedir) ─────────────────────────────────────────
$keyPassword = ''
$envFile = "$projectRoot\.env"
if (Test-Path $envFile) {
    $envLine = Select-String -Path $envFile -Pattern '^TAURI_SIGNING_PRIVATE_KEY_PASSWORD=' |
               Select-Object -First 1
    if ($envLine) {
        $keyPassword = ($envLine.Line -split '=', 2)[1].Trim()
        if ($keyPassword) { Write-Ok "Password leído desde .env" }
    }
}
if (-not $keyPassword) {
    $secPass = Read-Host "  Password de la clave de firma (Enter si vacío)" -AsSecureString
    $keyPassword = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secPass)
    )
}

# ── Compilar ─────────────────────────────────────────────────────────────────
Write-Step "Compilando la aplicación (puede tardar ~10 min)..."
$env:TAURI_SIGNING_PRIVATE_KEY          = $privateKey
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $keyPassword

Push-Location $projectRoot
try {
    npm run tauri build
    if ($LASTEXITCODE -ne 0) { Write-Fail "El build falló (código $LASTEXITCODE)" }
} finally {
    Pop-Location
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY          -ErrorAction SilentlyContinue
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
}
Write-Ok "Build exitoso"

# ── Localizar artefactos ─────────────────────────────────────────────────────
Write-Step "Localizando artefactos..."
$bundleDir = "$projectRoot\src-tauri\target\release\bundle\nsis"

$exeFile = Get-ChildItem "$bundleDir\*-setup.exe"     -ErrorAction SilentlyContinue | Select-Object -First 1
$zipFile = Get-ChildItem "$bundleDir\*.nsis.zip"      -ErrorAction SilentlyContinue |
           Where-Object { $_.Name -notlike '*.sig' }  | Select-Object -First 1
$sigFile = Get-ChildItem "$bundleDir\*.nsis.zip.sig"  -ErrorAction SilentlyContinue | Select-Object -First 1

if (-not $exeFile) { Write-Fail "No se encontró el instalador .exe en $bundleDir" }
if (-not $zipFile) { Write-Fail "No se encontró el .nsis.zip en $bundleDir" }
if (-not $sigFile) { Write-Fail "No se encontró el .sig en $bundleDir" }

Write-Ok "Instalador : $($exeFile.Name)"
Write-Ok "Updater    : $($zipFile.Name)"
Write-Ok "Firma      : $($sigFile.Name)"

# ── Generar latest.json ──────────────────────────────────────────────────────
Write-Step "Generando latest.json..."
$remoteUrl      = git -C $projectRoot remote get-url origin 2>&1
$repo           = $remoteUrl -replace '^.*github\.com[:/](.+?)(?:\.git)?$', '$1'
$signature      = (Get-Content $sigFile.FullName -Raw).Trim()
$zipNameEncoded = $zipFile.Name -replace ' ', '%20'
$downloadUrl    = "https://github.com/$repo/releases/download/$tag/$zipNameEncoded"
$pubDate        = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ssZ')

$latestObj = [ordered]@{
    version  = $version
    notes    = "Nueva versión disponible."
    pub_date = $pubDate
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = $signature
            url       = $downloadUrl
        }
    }
}
$latestJsonPath = "$projectRoot\latest.json"
($latestObj | ConvertTo-Json -Depth 5) | Set-Content $latestJsonPath -Encoding utf8
Write-Ok "latest.json generado"
Write-Ok "URL: $downloadUrl"

# ── Crear tag y pushear ──────────────────────────────────────────────────────
Write-Step "Creando y pusheando tag $tag..."
git -C $projectRoot tag $tag
git -C $projectRoot push origin $tag
Write-Ok "Tag $tag pusheado"

# ── Publicar release en GitHub ───────────────────────────────────────────────
Write-Step "Publicando release en GitHub..."
gh release create $tag `
    --repo $repo `
    --title "Lufal Ordenes $tag" `
    --notes "Nueva versión disponible." `
    $exeFile.FullName `
    $zipFile.FullName `
    $sigFile.FullName `
    $latestJsonPath

Write-Host "`n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host "  Release publicada exitosamente." -ForegroundColor Green
Write-Host "  https://github.com/$repo/releases/tag/$tag" -ForegroundColor DarkGray
Write-Host "  Las 8 computadoras detectarán la actualización al abrir la app." -ForegroundColor DarkGray
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`n" -ForegroundColor DarkGray
