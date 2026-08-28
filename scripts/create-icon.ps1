$ErrorActionPreference = "Stop"

# Release builds must use only repository-controlled branding assets.  Keeping
# the expected digests here prevents a file elsewhere on the developer machine
# from silently changing the packaged application icon.
$iconDir = Join-Path $PSScriptRoot "..\src-tauri\icons"
$expectedIcons = @{
  "icon.ico"  = "353E896F8CB3A12422569866FABB48BFE2C4CCFB38FA09698CFA009FCA0D28DC"
  "icon.icns" = "87F4813BEBC57451762394D2322C7353A833FEF7590886D9716F3171EA55A10B"
}

function Get-Sha256([string]$path) {
  $stream = [IO.File]::OpenRead($path)
  $sha256 = [Security.Cryptography.SHA256]::Create()
  try {
    return ([BitConverter]::ToString($sha256.ComputeHash($stream))).Replace("-", "")
  }
  finally {
    $sha256.Dispose()
    $stream.Dispose()
  }
}

foreach ($entry in $expectedIcons.GetEnumerator()) {
  $iconPath = Join-Path $iconDir $entry.Key
  if (-not (Test-Path -LiteralPath $iconPath -PathType Leaf)) {
    throw "Required repository icon is missing: $($entry.Key)"
  }

  $actualHash = Get-Sha256 $iconPath
  if (-not [string]::Equals($actualHash, $entry.Value, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Repository icon digest does not match the reviewed asset: $($entry.Key)"
  }

  Write-Host "Verified repository icon $($entry.Key)"
}
