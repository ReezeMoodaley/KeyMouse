$ErrorActionPreference = 'Stop'
$KeyMouseProject = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $KeyMouseProject
$KeyMouseDiagnosticDirectory = Join-Path $KeyMouseProject ('src-tauri\target\native-smoke-' + [Guid]::NewGuid().ToString('N'))
$KeyMousePreviousDiagnosticDirectory = $env:KEYMOUSE_DIAGNOSTIC_DATA_DIR
$KeyMousePreviousBrowserArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
New-Item -ItemType Directory -Path $KeyMouseDiagnosticDirectory | Out-Null
$env:KEYMOUSE_DIAGNOSTIC_DATA_DIR = $KeyMouseDiagnosticDirectory
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-address=127.0.0.1 --remote-debugging-port=9231'
$KeyMouseDiagnosticProcess = Start-Process -FilePath '.\src-tauri\target\debug\keymouse-diagnostics.exe' -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $KeyMouseDiagnosticDirectory 'stdout.log') -RedirectStandardError (Join-Path $KeyMouseDiagnosticDirectory 'stderr.log')
try {
    & node.exe .\scripts\native-smoke.mjs 9231
    if ($LASTEXITCODE -ne 0) { throw 'Native smoke tests failed' }
    if (-not $KeyMouseDiagnosticProcess.WaitForExit(5000)) { throw 'Diagnostic process did not shut down' }
} finally {
    if (-not $KeyMouseDiagnosticProcess.HasExited) { Stop-Process -Id $KeyMouseDiagnosticProcess.Id }
    if ($null -eq $KeyMousePreviousDiagnosticDirectory) { Remove-Item Env:\KEYMOUSE_DIAGNOSTIC_DATA_DIR -ErrorAction SilentlyContinue } else { $env:KEYMOUSE_DIAGNOSTIC_DATA_DIR = $KeyMousePreviousDiagnosticDirectory }
    if ($null -eq $KeyMousePreviousBrowserArguments) { Remove-Item Env:\WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue } else { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $KeyMousePreviousBrowserArguments }
}
