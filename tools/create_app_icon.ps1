Add-Type -AssemblyName System.Drawing

$assetDirectory = Join-Path $PSScriptRoot '..\assets'
$assetDirectory = [IO.Path]::GetFullPath($assetDirectory)

function New-AppIconBitmap([int] $size) {
    $bitmap = [Drawing.Bitmap]::new($size, $size, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.Clear([Drawing.Color]::Transparent)

    $scale = $size / 256.0
    $blueDark = [Drawing.Color]::FromArgb(255, 15, 94, 168)
    $blue = [Drawing.Color]::FromArgb(255, 47, 128, 209)
    $paleBlue = [Drawing.Color]::FromArgb(255, 191, 224, 250)
    $white = [Drawing.Color]::White

    $outer = [Drawing.RectangleF]::new(18 * $scale, 18 * $scale, 220 * $scale, 220 * $scale)
    $outerPath = [Drawing.Drawing2D.GraphicsPath]::new()
    $outerPath.AddArc($outer.X, $outer.Y, 104 * $scale, 104 * $scale, 180, 90)
    $outerPath.AddArc($outer.Right - 104 * $scale, $outer.Y, 104 * $scale, 104 * $scale, 270, 90)
    $outerPath.AddArc($outer.Right - 104 * $scale, $outer.Bottom - 104 * $scale, 104 * $scale, 104 * $scale, 0, 90)
    $outerPath.AddArc($outer.X, $outer.Bottom - 104 * $scale, 104 * $scale, 104 * $scale, 90, 90)
    $outerPath.CloseFigure()
    $graphics.FillPath([Drawing.SolidBrush]::new($blueDark), $outerPath)

    $drive = [Drawing.RectangleF]::new(36 * $scale, 56 * $scale, 184 * $scale, 118 * $scale)
    $drivePath = [Drawing.Drawing2D.GraphicsPath]::new()
    $drivePath.AddArc($drive.X, $drive.Y, 54 * $scale, 54 * $scale, 180, 90)
    $drivePath.AddArc($drive.Right - 54 * $scale, $drive.Y, 54 * $scale, 54 * $scale, 270, 90)
    $drivePath.AddArc($drive.Right - 54 * $scale, $drive.Bottom - 54 * $scale, 54 * $scale, 54 * $scale, 0, 90)
    $drivePath.AddArc($drive.X, $drive.Bottom - 54 * $scale, 54 * $scale, 54 * $scale, 90, 90)
    $drivePath.CloseFigure()
    $graphics.FillPath([Drawing.SolidBrush]::new($blue), $drivePath)

    $graphics.FillRectangle([Drawing.SolidBrush]::new($paleBlue), [Drawing.RectangleF]::new(54 * $scale, 78 * $scale, 92 * $scale, 12 * $scale))
    $graphics.FillEllipse([Drawing.SolidBrush]::new($white), [Drawing.RectangleF]::new(177 * $scale, 76 * $scale, 16 * $scale, 16 * $scale))

    $shield = [Drawing.Drawing2D.GraphicsPath]::new()
    $shield.AddPolygon([Drawing.PointF[]]@(
        [Drawing.PointF]::new(151 * $scale, 121 * $scale),
        [Drawing.PointF]::new(185 * $scale, 107 * $scale),
        [Drawing.PointF]::new(219 * $scale, 121 * $scale),
        [Drawing.PointF]::new(219 * $scale, 148 * $scale),
        [Drawing.PointF]::new(185 * $scale, 198 * $scale),
        [Drawing.PointF]::new(151 * $scale, 148 * $scale)
    ))
    $graphics.FillPath([Drawing.SolidBrush]::new($white), $shield)

    $checkPen = [Drawing.Pen]::new($blueDark, [Math]::Max(2.0, 11 * $scale))
    $checkPen.StartCap = [Drawing.Drawing2D.LineCap]::Round
    $checkPen.EndCap = [Drawing.Drawing2D.LineCap]::Round
    $checkPen.LineJoin = [Drawing.Drawing2D.LineJoin]::Round
    $graphics.DrawLines($checkPen, [Drawing.PointF[]]@(
        [Drawing.PointF]::new(168 * $scale, 146 * $scale),
        [Drawing.PointF]::new(180 * $scale, 158 * $scale),
        [Drawing.PointF]::new(203 * $scale, 133 * $scale)
    ))

    $checkPen.Dispose()
    $shield.Dispose()
    $drivePath.Dispose()
    $outerPath.Dispose()
    $graphics.Dispose()
    return $bitmap
}

$sizes = @(16, 32, 48, 64, 128, 256)
$pngs = @()
foreach ($size in $sizes) {
    $bitmap = New-AppIconBitmap $size
    $path = Join-Path $env:TEMP "linuxfs-manager-icon-$size.png"
    $bitmap.Save($path, [Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $pngs += [pscustomobject]@{ Size = $size; Bytes = [IO.File]::ReadAllBytes($path) }
}

[IO.File]::WriteAllBytes((Join-Path $assetDirectory 'linuxfs-manager.png'), $pngs[-1].Bytes)

$stream = [IO.MemoryStream]::new()
$writer = [IO.BinaryWriter]::new($stream)
$writer.Write([UInt16]0)
$writer.Write([UInt16]1)
$writer.Write([UInt16]$pngs.Count)
$offset = 6 + (16 * $pngs.Count)
foreach ($png in $pngs) {
    $dimension = if ($png.Size -eq 256) { [byte]0 } else { [byte]$png.Size }
    $writer.Write($dimension)
    $writer.Write($dimension)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]32)
    $writer.Write([UInt32]$png.Bytes.Length)
    $writer.Write([UInt32]$offset)
    $offset += $png.Bytes.Length
}
foreach ($png in $pngs) { $writer.Write($png.Bytes) }
[IO.File]::WriteAllBytes((Join-Path $assetDirectory 'linuxfs-manager.ico'), $stream.ToArray())
$writer.Dispose()
$stream.Dispose()

Get-Item (Join-Path $assetDirectory 'linuxfs-manager.png'), (Join-Path $assetDirectory 'linuxfs-manager.ico') |
    Select-Object Name, Length, LastWriteTime
