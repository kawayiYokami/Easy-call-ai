$vsDevCmd = "C:\BuildTools\Common7\Tools\VsDevCmd.bat"

if (-not (Test-Path $vsDevCmd)) {
  Write-Error "VsDevCmd not found at $vsDevCmd"
  exit 1
}

cmd /k "\"$vsDevCmd\" -arch=amd64"
