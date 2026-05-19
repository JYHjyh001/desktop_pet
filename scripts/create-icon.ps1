$ErrorActionPreference = "Stop"

$iconDir = Join-Path $PSScriptRoot "..\src-tauri\icons"
$iconPath = Join-Path $iconDir "icon.ico"

New-Item -ItemType Directory -Force -Path $iconDir | Out-Null

$width = 32
$height = 32
$pixelBytes = $width * $height * 4
$maskStride = 4
$maskBytes = $maskStride * $height
$bitmapHeaderBytes = 40
$imageBytes = $bitmapHeaderBytes + $pixelBytes + $maskBytes
$fileBytes = 6 + 16 + $imageBytes

$stream = [IO.MemoryStream]::new()
$writer = [IO.BinaryWriter]::new($stream)

function Write-ByteValue([int]$value) {
  $writer.Write([byte]$value)
}

function Write-UInt16Value([int]$value) {
  $writer.Write([uint16]$value)
}

function Write-UInt32Value([int]$value) {
  $writer.Write([uint32]$value)
}

function Write-Int32Value([int]$value) {
  $writer.Write([int32]$value)
}

# ICONDIR
Write-UInt16Value 0
Write-UInt16Value 1
Write-UInt16Value 1

# ICONDIRENTRY
Write-ByteValue $width
Write-ByteValue $height
Write-ByteValue 0
Write-ByteValue 0
Write-UInt16Value 1
Write-UInt16Value 32
Write-UInt32Value $imageBytes
Write-UInt32Value 22

# BITMAPINFOHEADER. ICO stores height as color bitmap + mask height.
Write-UInt32Value 40
Write-Int32Value $width
Write-Int32Value ($height * 2)
Write-UInt16Value 1
Write-UInt16Value 32
Write-UInt32Value 0
Write-UInt32Value $pixelBytes
Write-Int32Value 0
Write-Int32Value 0
Write-UInt32Value 0
Write-UInt32Value 0

# BGRA pixels, bottom-up. Draw a simple rounded blue-green mark.
for ($y = $height - 1; $y -ge 0; $y--) {
  for ($x = 0; $x -lt $width; $x++) {
    $dx = $x - 15.5
    $dy = $y - 15.5
    $distance = [Math]::Sqrt(($dx * $dx) + ($dy * $dy))
    $alpha = 0
    $red = 33
    $green = 104
    $blue = 232

    if ($distance -le 14.5) {
      $alpha = 255
      $green = [Math]::Min(232, 104 + ($y * 4))
      $blue = [Math]::Max(120, 232 - ($x * 2))
    }

    if ($distance -le 7.0) {
      $red = 255
      $green = 190
      $blue = 92
      $alpha = 255
    }

    $writer.Write([byte]$blue)
    $writer.Write([byte]$green)
    $writer.Write([byte]$red)
    $writer.Write([byte]$alpha)
  }
}

# AND mask. All zeros because alpha channel carries transparency.
for ($i = 0; $i -lt $maskBytes; $i++) {
  $writer.Write([byte]0)
}

$writer.Flush()

if ($stream.Length -ne $fileBytes) {
  throw "Generated icon has unexpected size $($stream.Length), expected $fileBytes"
}

[IO.File]::WriteAllBytes($iconPath, $stream.ToArray())
Write-Host "Created $iconPath"
