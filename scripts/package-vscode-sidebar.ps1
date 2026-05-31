param(
  [string]$OutputPath = "pai-test.vsix",
  [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$extensionDir = Join-Path $repoRoot "src\features\sidebar\extension"
$rootDistDir = Join-Path $repoRoot "dist"
$extensionDistDir = Join-Path $extensionDir "dist"
$rootPackageJsonPath = Join-Path $repoRoot "package.json"
$extensionPackageJsonPath = Join-Path $extensionDir "package.json"

if ([System.IO.Path]::IsPathRooted($OutputPath)) {
  $vsixPath = [System.IO.Path]::GetFullPath($OutputPath)
} else {
  $vsixPath = [System.IO.Path]::GetFullPath((Join-Path $extensionDir $OutputPath))
}

Push-Location $repoRoot
try {
  $syncVersionScript = @'
const fs = require("node:fs");
const rootPackagePath = process.argv[2];
const extensionPackagePath = process.argv[3];
const rootPackage = JSON.parse(fs.readFileSync(rootPackagePath, "utf8"));
const extensionPackage = JSON.parse(fs.readFileSync(extensionPackagePath, "utf8"));
extensionPackage.version = String(rootPackage.version || "");
fs.writeFileSync(extensionPackagePath, `${JSON.stringify(extensionPackage, null, 2)}\n`, "utf8");
'@
  $syncVersionScript | node - $rootPackageJsonPath $extensionPackageJsonPath
  if ($LASTEXITCODE -ne 0) {
    throw "Sync extension version failed with exit code $LASTEXITCODE."
  }

  if (-not $SkipBuild) {
    Write-Host "[vscode-sidebar] Building root frontend..."
    & pnpm build
    if ($LASTEXITCODE -ne 0) {
      throw "pnpm build failed with exit code $LASTEXITCODE."
    }
  }

  if (-not (Test-Path -LiteralPath $rootDistDir)) {
    throw "Root dist directory not found: $rootDistDir"
  }

  if (Test-Path -LiteralPath $extensionDistDir) {
    Remove-Item -LiteralPath $extensionDistDir -Recurse -Force
  }

  Write-Host "[vscode-sidebar] Syncing dist to extension workspace..."
  Copy-Item -LiteralPath $rootDistDir -Destination $extensionDir -Recurse -Force

  Push-Location $extensionDir
  try {
    Write-Host "[vscode-sidebar] Packaging VSIX..."
    $vsceArgs = @(
      "dlx",
      "@vscode/vsce",
      "package",
      "-o",
      $vsixPath,
      "--allow-missing-repository",
      "--skip-license"
    )
    & pnpm @vsceArgs
    if ($LASTEXITCODE -ne 0) {
      throw "vsce package failed with exit code $LASTEXITCODE."
    }
  }
  finally {
    Pop-Location
  }
}
finally {
  Pop-Location
}

Write-Host "[vscode-sidebar] Done: $vsixPath"
