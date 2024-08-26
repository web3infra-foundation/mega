# Check if both required arguments are provided
param (
    [Parameter(Mandatory=$true)][string]$baseDir,
    [Parameter(Mandatory=$true)][string]$configFile
)

# Function to clean and validate path
function Get-CleanPath {
    param ([string]$path)
    try {
        $fullPath = [System.IO.Path]::GetFullPath($path)
        if ($fullPath -notmatch '^[A-Za-z]:\\$') {
            $fullPath = $fullPath.TrimEnd('\')
        }
        [System.IO.Path]::GetFullPath($fullPath) | Out-Null
        return $fullPath
    }
    catch {
        Write-Error "Invalid path: $path"
        exit 1
    }
}

# Clean and validate paths
$baseDir = Get-CleanPath $baseDir
$configFile = Get-CleanPath $configFile

# Check if the config file exists
if (-not (Test-Path $configFile)) {
    Write-Error "Error: Config file not found at $configFile"
    exit 1
}

# Check if the mono directory already exists
$monoDir = Join-Path $baseDir "mono"
if (Test-Path $monoDir) {
    $confirm = Read-Host "The directory $monoDir already exists. Do you want to delete it and continue? (y/N)"
    if ($confirm -eq "y" -or $confirm -eq "Y") {
        Write-Host "Deleting existing mono directory."
        Remove-Item -Path $monoDir -Recurse -Force -ErrorAction Stop
    }
    else {
        Write-Host "Operation cancelled. Exiting..."
        exit 0
    }
}

# Create directory structure
try {
    $monoDataDir = Join-Path $monoDir "mono-data"
    $pgDataDir = Join-Path $monoDir "pg-data"

    New-Item -ItemType Directory -Force -Path $monoDir -ErrorAction Stop | Out-Null
    New-Item -ItemType Directory -Force -Path $monoDataDir -ErrorAction Stop | Out-Null
    New-Item -ItemType Directory -Force -Path $pgDataDir -ErrorAction Stop | Out-Null

    $subDirs = @("etc", "cache", "lfs", "logs", "objects")
    foreach ($dir in $subDirs) {
        New-Item -ItemType Directory -Force -Path (Join-Path $monoDataDir $dir) -ErrorAction Stop | Out-Null
    }

    $sshDir = Join-Path $monoDataDir "etc\ssh"
    $httpsDir = Join-Path $monoDataDir "etc\https"
    New-Item -ItemType Directory -Force -Path $sshDir -ErrorAction Stop | Out-Null
    New-Item -ItemType Directory -Force -Path $httpsDir -ErrorAction Stop | Out-Null
}
catch {
    Write-Error "Failed to create directory structure: $_"
    exit 1
}

# Generate SSH key for sshd (non-interactive)
$sshKeyFile = Join-Path $sshDir "ssh_host_rsa_key"
if (Get-Command "ssh-keygen" -ErrorAction SilentlyContinue) {
    $null = ssh-keygen -t rsa -b 4096 -f $sshKeyFile -N '""' -C "sshd host key" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "SSH host key generated at $sshKeyFile"
    }
    else {
        Write-Warning "Failed to generate SSH host key"
    }
}
else {
    Write-Warning "ssh-keygen not found. Skipping SSH key generation."
}

# Copy config file
try {
    Copy-Item -Path $configFile -Destination (Join-Path $monoDataDir "etc\config.toml") -ErrorAction Stop
    Write-Host "Config file copied to $((Join-Path $monoDataDir 'etc\config.toml'))"
}
catch {
    Write-Error "Failed to copy config file: $_"
    exit 1
}

# Display the created directory structure
Write-Host "`nDirectory structure has been successfully created."
Write-Host "`nNote: Please review and set appropriate permissions if needed."