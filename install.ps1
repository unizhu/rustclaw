# RustClaw One-Click Install Script for Windows
# Usage: iex (irm https://raw.githubusercontent.com/unizhu/rustclaw/main/install.ps1)

param(
    [string]$InstallDir = "",
    [string]$Version = "latest"
)

# Colors for output
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

Write-ColorOutput Green "🚀 RustClaw Installer for Windows"
Write-Output "Detected OS: Windows"
Write-Output "Detected Architecture: $env:PROCESSOR_ARCHITECTURE"

# Determine architecture
if ($env:PROCESSOR_ARCHITECTURE -eq "AMD64") {
    $Target = "x86_64-pc-windows-msvc"
} else {
    Write-ColorOutput Red "✗ Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
    exit 1
}

Write-Output "Target: $Target"

# Set installation directory
if ($InstallDir -eq "") {
    $InstallDir = "$env:LOCALAPPDATA\RustClaw"
}

$BinaryName = "rustclaw-gateway.exe"
$ArchiveName = "rustclaw-${Target}.zip"

if ($Version -eq "latest") {
    $DownloadUrl = "https://github.com/unizhu/rustclaw/releases/latest/download/${ArchiveName}"
} else {
    $DownloadUrl = "https://github.com/unizhu/rustclaw/releases/download/${Version}/${ArchiveName}"
}

# Create installation directory
if (-not (Test-Path $InstallDir)) {
    Write-ColorOutput Yellow "Creating installation directory: $InstallDir"
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Download and extract
$TempFile = Join-Path $env:TEMP $ArchiveName

Write-ColorOutput Yellow "`nDownloading RustClaw..."
try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempFile -UseBasicParsing
} catch {
    Write-ColorOutput Red "✗ Download failed: $_"
    exit 1
}

Write-ColorOutput Yellow "Extracting..."
try {
    Expand-Archive -Path $TempFile -DestinationPath $InstallDir -Force
} catch {
    Write-ColorOutput Red "✗ Extraction failed: $_"
    exit 1
}

# Clean up
Remove-Item $TempFile -Force

# Add to PATH if not already there
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-ColorOutput Yellow "Adding to PATH..."
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$InstallDir", "User")
    $env:PATH = "$env:PATH;$InstallDir"
}

# Verify installation
$BinaryPath = Join-Path $InstallDir $BinaryName
if (Test-Path $BinaryPath) {
    Write-ColorOutput Green "`n✓ RustClaw installed successfully!"
    Write-Output "  Location: $BinaryPath"
    Write-Output ""
    Write-ColorOutput Yellow "Next steps:"
    Write-Output "  1. Set your Telegram bot token:"
    Write-Output "     `$env:TELEGRAM_BOT_TOKEN = 'your_token_here'"
    Write-Output ""
    Write-Output "  2. Set your OpenAI API key:"
    Write-Output "     `$env:OPENAI_API_KEY = 'your_key_here'"
    Write-Output ""
    Write-Output "  3. Run RustClaw:"
    Write-Output "     rustclaw-gateway"
    Write-Output ""
    Write-ColorOutput Green "🎉 Installation complete!"
} else {
    Write-ColorOutput Red "`n✗ Installation failed"
    exit 1
}
