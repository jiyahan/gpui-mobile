param(
    [string]$SdkRoot = $env:OHOS_SDK_HOME
)

$ErrorActionPreference = 'Stop'
if (-not $SdkRoot) {
    throw 'Set OHOS_SDK_HOME to the OHOS native SDK directory.'
}
$SdkRoot = (Resolve-Path -LiteralPath $SdkRoot).Path
$clang = Join-Path $SdkRoot 'llvm\bin\clang.exe'
$sysroot = Join-Path $SdkRoot 'sysroot'
$ar = Join-Path $SdkRoot 'llvm\bin\llvm-ar.exe'
foreach ($path in @($clang, $sysroot, $ar)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing OHOS SDK path: $path" }
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$scratch = Join-Path $repoRoot 'temp\ohos-build'
New-Item -ItemType Directory -Force -Path $scratch | Out-Null
$linker = Join-Path $scratch 'ohos-linker.cmd'
[System.IO.File]::WriteAllText($linker,
    "@echo off`r`n`"$clang`" -target aarch64-linux-ohos --sysroot=`"$sysroot`" %*`r`n")

$env:CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER = $linker
$env:CC_aarch64_unknown_linux_ohos = $linker
$env:AR_aarch64_unknown_linux_ohos = $ar
$env:CARGO_TARGET_DIR = Join-Path $scratch 'target'
$manifest = Join-Path $PSScriptRoot 'rust\Cargo.toml'

Push-Location $repoRoot
try {
    cargo build --manifest-path $manifest --release --target aarch64-unknown-linux-ohos --lib
    if ($LASTEXITCODE -ne 0) { throw "Cargo build failed: $LASTEXITCODE" }
} finally {
    Pop-Location
}

$library = Join-Path $env:CARGO_TARGET_DIR 'aarch64-unknown-linux-ohos\release\libgpui_mobile_ohos_example.a'
$destination = Join-Path $PSScriptRoot 'entry\libs\arm64-v8a\libgpui_mobile_ohos_example.a'
Copy-Item -LiteralPath $library -Destination $destination -Force
$oldLibrary = Join-Path $PSScriptRoot 'entry\libs\arm64-v8a\libgpui_mobile.so'
if (Test-Path -LiteralPath $oldLibrary) { Remove-Item -LiteralPath $oldLibrary -Force }
Write-Host "Copied $destination"
