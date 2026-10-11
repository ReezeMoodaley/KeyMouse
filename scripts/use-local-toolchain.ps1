# Dot-source this script in the implementation workspace. Normal Rust installations
# elsewhere can use the README's standard commands without this helper.
$KeyMouseToolchainRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\.toolchain'))
$KeyMouseCargoExecutable = Join-Path $KeyMouseToolchainRoot 'cargo\bin\cargo.exe'
if (Test-Path -LiteralPath $KeyMouseCargoExecutable) {
    $env:CARGO_HOME = Join-Path $KeyMouseToolchainRoot 'cargo'
    $env:RUSTUP_HOME = Join-Path $KeyMouseToolchainRoot 'rustup'
    $env:PATH = "$(Join-Path $env:CARGO_HOME 'bin');$env:PATH"
    Write-Output "Using workspace Rust toolchain: $KeyMouseToolchainRoot"
} elseif (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Install stable Rust MSVC and Visual Studio C++ Build Tools, as described in README.md.'
}
