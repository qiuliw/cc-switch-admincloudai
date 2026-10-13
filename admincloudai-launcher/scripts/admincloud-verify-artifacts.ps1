param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$Installer,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [string]$SevenZip = 'C:/Program Files/7-Zip/7z.exe'
)
$ErrorActionPreference = 'Stop'
$exePath = (Resolve-Path -LiteralPath $Executable).Path
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
$payloadPath = Join-Path $outputPath 'nsis-payload'
New-Item -ItemType Directory -Force -Path $payloadPath | Out-Null
& $SevenZip x $installerPath "-o$payloadPath" -y | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'NSIS extraction failed.' }
$payload = @(Get-ChildItem -LiteralPath $payloadPath -Recurse -File -Filter cc-switch.exe)
if ($payload.Count -ne 1) { throw "Expected exactly one application executable, got $($payload.Count)." }
$exeHash = (Get-FileHash -LiteralPath $exePath -Algorithm SHA256).Hash
$payloadHash = (Get-FileHash -LiteralPath $payload[0].FullName -Algorithm SHA256).Hash
if ($exeHash -ne $payloadHash) { throw 'NSIS application payload differs from release executable.' }
$version = (Get-Item -LiteralPath $exePath).VersionInfo
if ($version.ProductName -ne 'AdminCloudAI 启动器' -or $version.ProductVersion -ne '4.0.9') {
    throw "Incorrect product identity: $($version.ProductName) / $($version.ProductVersion)"
}
$resourcePath = Join-Path $outputPath 'exe-resources'
& $SevenZip x $exePath "-o$resourcePath" '.rsrc\ICON\*' -y | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'Executable icon extraction failed.' }
$iconPath = Join-Path $PSScriptRoot '../src-tauri/icons/icon.ico'
$iconBytes = [System.IO.File]::ReadAllBytes($iconPath)
$iconCount = [BitConverter]::ToUInt16($iconBytes, 4)
$expectedIcons = @()
for ($index = 0; $index -lt $iconCount; $index++) {
    $entry = 6 + 16 * $index
    $size = [BitConverter]::ToUInt32($iconBytes, $entry + 8)
    $offset = [BitConverter]::ToUInt32($iconBytes, $entry + 12)
    $frameBytes = New-Object byte[] $size
    [Array]::Copy($iconBytes, $offset, $frameBytes, 0, $size)
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try { $expectedIcons += [BitConverter]::ToString($hasher.ComputeHash($frameBytes)).Replace('-', '') }
    finally { $hasher.Dispose() }
}
$actualIcons = @(Get-ChildItem -LiteralPath (Join-Path $resourcePath '.rsrc/ICON') -File |
    ForEach-Object { (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash })
if ($actualIcons.Count -ne $iconCount -or @(Compare-Object ($expectedIcons | Sort-Object) ($actualIcons | Sort-Object)).Count -ne 0) {
    throw 'Executable icon resources differ from converted product ICO.'
}
$metadata = [ordered]@{
    productName = $version.ProductName
    productVersion = $version.ProductVersion
    fileVersion = $version.FileVersion
    executable = [System.IO.Path]::GetFileName($exePath)
    executableBytes = (Get-Item -LiteralPath $exePath).Length
    executableSHA256 = $exeHash
    installer = [System.IO.Path]::GetFileName($installerPath)
    installerBytes = (Get-Item -LiteralPath $installerPath).Length
    installerSHA256 = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash
    payloadSHA256 = $payloadHash
    payloadMatches = $true
    iconFrames = $iconCount
    iconResourcesMatch = $true
    convertedIconSHA256 = (Get-FileHash -LiteralPath $iconPath -Algorithm SHA256).Hash
    authenticode = (Get-AuthenticodeSignature -LiteralPath $exePath).Status.ToString()
}
$metadata | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $outputPath 'artifact-verification.json') -Encoding utf8
$metadata | ConvertTo-Json
