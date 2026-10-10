# Install the Shum CLI preview for the current Windows user.
#   irm https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.ps1 | iex
# Uninstall:
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.ps1))) -Uninstall
param([switch]$Uninstall)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$repository = 'hiTechTeam/Shum-CLI'
$projectUrl = "https://github.com/$repository"
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\Shum'
$binary = Join-Path $installDir 'shum.exe'
$marker = Join-Path $installDir '.shum-installer'
$lock = Join-Path $env:LOCALAPPDATA 'Programs\.shum-install.lock'

function Fail([string]$message) { throw "Shum: $message" }

function Remove-FromUserPath([string]$directory) {
    $current = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $current) { return }
    $kept = $current.Split(';') | Where-Object { $_ -and ($_.TrimEnd('\') -ne $directory.TrimEnd('\')) }
    [Environment]::SetEnvironmentVariable('Path', ($kept -join ';'), 'User')
}

# Refuse to overwrite a directory this installer did not create.
function Assert-Owned {
    if (Test-Path -LiteralPath $installDir) {
        if (-not (Test-Path -LiteralPath $marker -PathType Leaf) -or
            ((Get-Content -LiteralPath $marker -Raw).Trim() -ne 'shum-install-v1')) {
            Fail "Каталог $installDir не принадлежит этому установщику"
        }
    }
}

if ([Environment]::OSVersion.Version.Major -lt 10) { Fail "Нужна Windows 10 или новее. Информация: $projectUrl" }

$architecture = try { [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() } catch { $env:PROCESSOR_ARCHITECTURE }
switch ($architecture) {
    { $_ -in 'X64', 'AMD64' } { $arch = 'x64' }
    'Arm64' { $arch = 'arm64' }
    default { Fail "Архитектура $architecture не поддерживается. Информация: $projectUrl" }
}
$asset = "shum-windows-$arch.zip"

New-Item -ItemType Directory -Force -Path (Split-Path $lock) | Out-Null
try { New-Item -ItemType Directory -Path $lock -ErrorAction Stop | Out-Null }
catch { Fail "Другой установщик работает. После аварийного завершения удалите $lock вручную." }

$temporary = $null
try {
    Assert-Owned

    if ($Uninstall) {
        if (Test-Path -LiteralPath $binary) {
            & $binary daemon --uninstall
            if ($LASTEXITCODE -ne 0) { Fail 'Не удалось остановить службы Shum' }
        }
        if (Test-Path -LiteralPath $installDir) { Remove-Item -LiteralPath $installDir -Recurse -Force }
        Remove-FromUserPath $installDir
        Write-Host 'Shum удалён. Данные профилей и ключи в диспетчере учётных данных Windows сохранены.'
        return
    }

    # Pick the newest published release that carries a Windows build.
    $releases = Invoke-RestMethod -UseBasicParsing -Headers @{ 'User-Agent' = 'shum-install' } `
        -Uri "https://api.github.com/repos/$repository/releases?per_page=30"
    $release = $releases | Where-Object { -not $_.draft -and ($_.assets.name -contains $asset) } | Select-Object -First 1
    if (-not $release) { Fail "Сборка $asset не найдена. Информация: $projectUrl" }
    $base = "https://github.com/$repository/releases/download/$($release.tag_name)"

    $temporary = Join-Path ([IO.Path]::GetTempPath()) ('shum-install-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $temporary | Out-Null
    $archive = Join-Path $temporary $asset
    $checksum = Join-Path $temporary 'checksum'
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile $archive
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset.sha256" -OutFile $checksum

    $fields = (Get-Content -LiteralPath $checksum -Raw).Trim() -split '\s+'
    if ($fields.Count -ne 2 -or $fields[1] -ne $asset -or $fields[0] -notmatch '^[0-9a-fA-F]{64}$') { Fail 'Недействительный файл SHA-256' }
    $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
    if ($actual -ne $fields[0].ToUpperInvariant()) { Fail 'SHA-256 не совпадает. Установка отменена.' }

    # The archive holds exactly these files, with no folders or traversal paths.
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($archive)
    try { $entries = @($zip.Entries | ForEach-Object { $_.FullName } | Sort-Object) } finally { $zip.Dispose() }
    if (($entries -join ',') -ne 'LICENSE,shum.exe') { Fail 'Неожиданное содержимое архива' }
    $unpacked = Join-Path $temporary 'unpacked'
    Expand-Archive -LiteralPath $archive -DestinationPath $unpacked
    $version = (& (Join-Path $unpacked 'shum.exe') --version)
    if ($version -notmatch '^shum \d+\.\d+\.\d+$') { Fail 'Архив содержит неработающую программу' }

    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    Set-Content -LiteralPath $marker -Value 'shum-install-v1' -Encoding ASCII
    # A running service keeps the old file open: Windows allows renaming it, not replacing it.
    if (Test-Path -LiteralPath $binary) { Move-Item -LiteralPath $binary -Destination "$binary.$([Guid]::NewGuid().ToString('N')).old" -Force }
    Copy-Item -LiteralPath (Join-Path $unpacked 'shum.exe') -Destination $binary
    Copy-Item -LiteralPath (Join-Path $unpacked 'LICENSE') -Destination (Join-Path $installDir 'LICENSE') -Force
    Get-ChildItem -LiteralPath $installDir -Filter 'shum.exe.*.old' | Remove-Item -Force -ErrorAction SilentlyContinue

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not (($userPath -split ';') | Where-Object { $_.TrimEnd('\') -eq $installDir.TrimEnd('\') })) {
        [Environment]::SetEnvironmentVariable('Path', (@($userPath, $installDir) | Where-Object { $_ }) -join ';', 'User')
    }
    if (-not (($env:Path -split ';') -contains $installDir)) { $env:Path = "$env:Path;$installDir" }

    # Refresh active profiles; a first install never opens keys or creates a profile.
    & $binary daemon --refresh | Out-Null
    Write-Host "$($version -replace '^shum ', 'Shum ') установлен. Запуск: shum"
    Write-Host 'Если команда не найдена, откройте новое окно терминала.'
}
finally {
    if ($temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue }
    Remove-Item -LiteralPath $lock -Force -ErrorAction SilentlyContinue
}
