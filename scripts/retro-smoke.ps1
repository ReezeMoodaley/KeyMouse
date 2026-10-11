$ErrorActionPreference = 'Stop'
$KeyMouseProject = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $KeyMouseProject
$KeyMouseUiDirectory = Join-Path $KeyMouseProject ('src-tauri\target\retro-smoke-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $KeyMouseUiDirectory | Out-Null
# This disposable instance never enables keyboard-driven movement.
Set-Content -LiteralPath (Join-Path $KeyMouseUiDirectory 'config.json') -Value '{"version":1,"enabled":false,"start_minimized":true,"start_at_login":false,"theme":"system","bindings":{"up":124,"down":125,"left":126,"right":127,"left_button":128,"right_button":129,"middle_button":130,"scroll_positive":131,"scroll_negative":132,"precision":133,"turbo":134,"horizontal":135},"normal":{"min_speed":40,"max_speed":1400,"ramp_ms":900,"exponent":1.6},"precision":{"min_speed":8,"max_speed":80,"ramp_ms":700,"exponent":1.8,"hold_ms":160,"step":1},"turbo":{"min_speed":900,"max_speed":4000,"ramp_ms":350,"exponent":1.2},"scroll":{"repeat":true,"delay_ms":300,"interval_ms":70,"amount":120},"grid":{"key":71,"depth":3}}' -Encoding ASCII
$KeyMouseOldDirectory = $env:KEYMOUSE_DIAGNOSTIC_DATA_DIR
$KeyMouseOldArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$env:KEYMOUSE_DIAGNOSTIC_DATA_DIR = $KeyMouseUiDirectory
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-address=127.0.0.1 --remote-debugging-port=9232'
try {
    for ($KeyMouseUiPass = 0; $KeyMouseUiPass -lt 2; $KeyMouseUiPass++) {
        $KeyMouseUiProcess = Start-Process -FilePath '.\src-tauri\target\debug\keymouse-diagnostics.exe' -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $KeyMouseUiDirectory "stdout-$KeyMouseUiPass.log") -RedirectStandardError (Join-Path $KeyMouseUiDirectory "stderr-$KeyMouseUiPass.log")
        try {
            & node.exe .\scripts\retro-smoke.mjs 9232 $KeyMouseUiDirectory $KeyMouseUiPass
            if ($LASTEXITCODE -ne 0) { throw 'Retro UI smoke checks failed' }
            if (-not $KeyMouseUiProcess.WaitForExit(5000)) { throw 'UI instance did not shut down' }
        } finally { if (-not $KeyMouseUiProcess.HasExited) { Stop-Process -Id $KeyMouseUiProcess.Id } }
    }
    Write-Output "Retro UI artifacts: $KeyMouseUiDirectory"
} finally {
    if ($null -eq $KeyMouseOldDirectory) { Remove-Item Env:\KEYMOUSE_DIAGNOSTIC_DATA_DIR -ErrorAction SilentlyContinue } else { $env:KEYMOUSE_DIAGNOSTIC_DATA_DIR = $KeyMouseOldDirectory }
    if ($null -eq $KeyMouseOldArguments) { Remove-Item Env:\WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue } else { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $KeyMouseOldArguments }
}
