#Requires -Version 5.1

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$DefaultConfig = [ordered]@{
    "audio_format"          = "m4a"
    "audio_quality"         = "0"
    "cover_mode_non_square" = "stretch"
    "auto_crop_shorts"      = $true
    "embed_lyrics"          = $true
    "concurrent_fragments"  = 5
    "output_folder"         = "output"
    "bin_folder"            = "bin"
    "private_folder"        = "private"
    "temp_folder"           = "temp_download"
}

function Load-AppConfig {
    $configPath = Join-Path $PSScriptRoot "config.json"
    $config = $DefaultConfig

    if (Test-Path $configPath) {
        try {
            $json = Get-Content $configPath -Raw -Encoding UTF8 | ConvertFrom-Json
            foreach ($prop in $json.PSObject.Properties) {
                $config[$prop.Name] = $prop.Value
            }
        } catch {
            Write-Host "  [!] Warning: Failed to read config.json, using defaults." -ForegroundColor Yellow
        }
    } else {
        $jsonContent = $DefaultConfig | ConvertTo-Json -Depth 4
        Set-Content -Path $configPath -Value $jsonContent -Encoding UTF8
    }

    return $config
}

$Script:Config = Load-AppConfig
$Script:Config["YtDlpUrl"]         = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
$Script:Config["AtomicParsleyUrl"] = "https://github.com/wez/atomicparsley/releases/download/20240608.083822.1ed9031/AtomicParsleyWindows.zip"
$Script:Config["FfmpegUrl"]        = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"

function Find-Executable {
    param([string]$ExeName)

    $binFolder = if ($Script:Config["bin_folder"]) { $Script:Config["bin_folder"] } else { "bin" }
    $binPath = Join-Path $PSScriptRoot "$binFolder\$ExeName.exe"
    if (Test-Path $binPath) { return $binPath }

    $pathResult = Get-Command $ExeName -ErrorAction SilentlyContinue
    if ($pathResult -and (Test-Path $pathResult.Source)) { return $pathResult.Source }

    $searchPaths = @(
        "$env:LOCALAPPDATA\Programs\$ExeName\$ExeName.exe",
        "$env:LOCALAPPDATA\$ExeName\$ExeName.exe",
        "${env:ProgramFiles}\$ExeName\$ExeName.exe",
        "${env:ProgramFiles(x86)}\$ExeName\$ExeName.exe"
    )
    foreach ($p in $searchPaths) {
        if (Test-Path $p) { return $p }
    }

    return $null
}

function Ensure-BinDirectory {
    $binFolder = if ($Script:Config["bin_folder"]) { $Script:Config["bin_folder"] } else { "bin" }
    $binDir = Join-Path $PSScriptRoot $binFolder
    if (-not (Test-Path $binDir)) {
        New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    }
    return $binDir
}

function Install-YtDlpTool {
    $binDir = Ensure-BinDirectory
    $targetPath = Join-Path $binDir "yt-dlp.exe"

    Write-Host "  -> Downloading yt-dlp from official releases..." -ForegroundColor Gray
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $Script:Config["YtDlpUrl"] -OutFile $targetPath -UseBasicParsing
        if (Test-Path $targetPath) {
            Write-Host "  [OK] yt-dlp installed to bin folder." -ForegroundColor Green
            return $targetPath
        }
    } catch {
        Write-Host "  [ERROR] Failed to download yt-dlp: $($_.Exception.Message)" -ForegroundColor Red
    }
    return $null
}

function Install-AtomicParsleyTool {
    $binDir = Ensure-BinDirectory
    $targetPath = Join-Path $binDir "AtomicParsley.exe"
    $zipPath = Join-Path ([System.IO.Path]::GetTempPath()) "AtomicParsley.zip"
    $tempExtract = Join-Path ([System.IO.Path]::GetTempPath()) "AtomicParsley_ext"

    Write-Host "  -> Downloading AtomicParsley..." -ForegroundColor Gray
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $Script:Config["AtomicParsleyUrl"] -OutFile $zipPath -UseBasicParsing

        if (Test-Path $tempExtract) { Remove-Item $tempExtract -Recurse -Force }
        Expand-Archive -LiteralPath $zipPath -DestinationPath $tempExtract -Force

        $foundExe = Get-ChildItem -Path $tempExtract -Filter "AtomicParsley.exe" -Recurse | Select-Object -First 1
        if ($foundExe) {
            Move-Item -LiteralPath $foundExe.FullName -Destination $targetPath -Force
            Write-Host "  [OK] AtomicParsley installed to bin folder." -ForegroundColor Green
            return $targetPath
        }
    } catch {
        Write-Host "  [ERROR] Failed to download AtomicParsley: $($_.Exception.Message)" -ForegroundColor Red
    } finally {
        Remove-Item $zipPath, $tempExtract -Recurse -Force -ErrorAction SilentlyContinue
    }
    return $null
}

function Install-FfmpegTool {
    $binDir = Ensure-BinDirectory
    $targetPath = Join-Path $binDir "ffmpeg.exe"
    $zipPath = Join-Path ([System.IO.Path]::GetTempPath()) "ffmpeg_essentials.zip"
    $tempExtract = Join-Path ([System.IO.Path]::GetTempPath()) "ffmpeg_ext"

    Write-Host "  -> Downloading ffmpeg essentials (this may take 1-2 minutes)..." -ForegroundColor Gray
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $Script:Config["FfmpegUrl"] -OutFile $zipPath -UseBasicParsing

        if (Test-Path $tempExtract) { Remove-Item $tempExtract -Recurse -Force }
        Expand-Archive -LiteralPath $zipPath -DestinationPath $tempExtract -Force

        $foundFfmpeg = Get-ChildItem -Path $tempExtract -Filter "ffmpeg.exe" -Recurse | Select-Object -First 1
        if ($foundFfmpeg) {
            Move-Item -LiteralPath $foundFfmpeg.FullName -Destination $targetPath -Force
            Write-Host "  [OK] ffmpeg installed to bin folder." -ForegroundColor Green
        }
        $foundFfprobe = Get-ChildItem -Path $tempExtract -Filter "ffprobe.exe" -Recurse | Select-Object -First 1
        if ($foundFfprobe) {
            $probeTarget = Join-Path $binDir "ffprobe.exe"
            Move-Item -LiteralPath $foundFfprobe.FullName -Destination $probeTarget -Force
        }
        if (Test-Path $targetPath) { return $targetPath }
    } catch {
        Write-Host "  [ERROR] Failed to download ffmpeg: $($_.Exception.Message)" -ForegroundColor Red
    } finally {
        Remove-Item $zipPath, $tempExtract -Recurse -Force -ErrorAction SilentlyContinue
    }
    return $null
}

function Initialize-Tools {
    Write-Host "Checking required tools..." -ForegroundColor Cyan

    $yt = Find-Executable -ExeName "yt-dlp"
    if (-not $yt) {
        Write-Host "  * yt-dlp not found. Auto-installing..." -ForegroundColor Yellow
        $yt = Install-YtDlpTool
    } else {
        Write-Host "  * yt-dlp: $yt" -ForegroundColor Gray
    }

    $ff = Find-Executable -ExeName "ffmpeg"
    if (-not $ff) {
        Write-Host "  * ffmpeg not found. Auto-installing..." -ForegroundColor Yellow
        $ff = Install-FfmpegTool
    } else {
        Write-Host "  * ffmpeg: $ff" -ForegroundColor Gray
    }

    $probe = Find-Executable -ExeName "ffprobe"
    if ($probe) {
        Write-Host "  * ffprobe: $probe" -ForegroundColor Gray
    }

    $ap = Find-Executable -ExeName "AtomicParsley"
    if (-not $ap) {
        Write-Host "  * AtomicParsley not found. Auto-installing..." -ForegroundColor Yellow
        $ap = Install-AtomicParsleyTool
    } else {
        Write-Host "  * AtomicParsley: $ap" -ForegroundColor Gray
    }

    if (-not $yt -or -not $ff) {
        Write-Host "`n[FATAL] Missing required core tools. Please check your setup." -ForegroundColor Red
        return $null
    }

    return @{
        YtDlp         = $yt
        Ffmpeg        = $ff
        Ffprobe       = $probe
        AtomicParsley = $ap
    }
}

function Get-CookieArguments {
    $privFolder = if ($Script:Config["private_folder"]) { $Script:Config["private_folder"] } else { "private" }
    $privateDir = Join-Path $PSScriptRoot $privFolder
    if (-not (Test-Path $privateDir)) { return @() }

    $ytCookie = Join-Path $privateDir "www.youtube.com_cookies.txt"
    if (Test-Path $ytCookie) {
        Write-Host "  [Cookie] YouTube specific cookie detected: www.youtube.com_cookies.txt" -ForegroundColor DarkCyan
        return @("--cookies", $ytCookie)
    }

    $globalCookie = Join-Path $privateDir "cookies.txt"
    if (Test-Path $globalCookie) {
        Write-Host "  [Cookie] Global cookie detected: cookies.txt" -ForegroundColor DarkCyan
        return @("--cookies", $globalCookie)
    }

    return @()
}

function Normalize-Url {
    param([string]$RawUrl)

    $u = $RawUrl.Trim()
    if ($u -match '^ttps?://') {
        $u = "h" + $u
    } elseif ($u -notmatch '^[a-zA-Z]+://') {
        $u = "https://" + $u
    }

    if ($u -match 'youtube\.com/watch\?v=([^&]+)' -and $u -match 'list=(?:RD|UL|TL)') {
        $videoId = $Matches[1]
        $u = "https://www.youtube.com/watch?v=$videoId"
    }

    return $u
}

function Build-SiteSpecificArguments {
    param(
        [string]$Url,
        [string]$FfmpegPath,
        [string]$AudioFormat,
        [string]$AudioQuality,
        [int]$ConcurrentFrags,
        [bool]$EmbedLyrics
    )

    $args = @(
        "--newline",
        "--no-playlist",
        "-N", [string]$ConcurrentFrags,
        "--ffmpeg-location", $FfmpegPath,
        "-x", "--audio-format", $AudioFormat, "--audio-quality", [string]$AudioQuality,
        "--write-thumbnail", "--convert-thumbnails", "jpg",
        "--write-info-json", "--no-embed-chapters"
    )

    if ($EmbedLyrics) {
        $args += @("--write-subs", "--sub-langs", "ja,en,ja-JP,en-US", "--sub-format", "vtt/lrc/best")
    }

    if ($Url -match "youtube\.com|youtu\.be") {
        Write-Host "  [Site] Detected: YouTube / YouTube Music" -ForegroundColor DarkGray
        $args += @("--extractor-args", "youtube:player_client=default,-1")
    } elseif ($Url -match "twitter\.com|x\.com") {
        Write-Host "  [Site] Detected: X (Twitter)" -ForegroundColor DarkGray
    } elseif ($Url -match "tiktok\.com") {
        Write-Host "  [Site] Detected: TikTok" -ForegroundColor DarkGray
        $args += @(
            "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            "--referer", "https://www.tiktok.com/"
        )
    } elseif ($Url -match "nicovideo\.jp|nico\.ms") {
        Write-Host "  [Site] Detected: Niconico" -ForegroundColor DarkGray
    } else {
        Write-Host "  [Site] Detected: Generic Web Source" -ForegroundColor DarkGray
    }

    return $args
}

function Get-ImageDimensions {
    param(
        [string]$ImagePath,
        [string]$FfprobePath
    )

    if ($FfprobePath -and (Test-Path $FfprobePath)) {
        $dim = & $FfprobePath -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0 $ImagePath 2>$null
        if ($dim -match '^(?<w>\d+)x(?<h>\d+)$') {
            return @{
                Width  = [int]$Matches['w']
                Height = [int]$Matches['h']
            }
        }
    }

    try {
        Add-Type -AssemblyName System.Drawing -ErrorAction SilentlyContinue
        $img = [System.Drawing.Image]::FromFile($ImagePath)
        $w = $img.Width
        $h = $img.Height
        $img.Dispose()
        return @{ Width = $w; Height = $h }
    } catch {
        return $null
    }
}

function Format-CoverArt {
    param(
        [string]$FfmpegPath,
        [string]$FfprobePath,
        [string]$InputImage,
        [string]$OutputImage,
        [string]$NonSquareMode
    )

    $dims = Get-ImageDimensions -ImagePath $InputImage -FfprobePath $FfprobePath
    if ($dims) {
        Write-Host "    [Cover Info] Dimension: $($dims.Width)x$($dims.Height)" -ForegroundColor DarkGray
        if ($dims.Width -eq $dims.Height) {
            Write-Host "    [Cover Action] Image is already 1:1 square! Using original without re-encoding." -ForegroundColor Green
            return $InputImage
        }
    }

    if ($NonSquareMode -eq "original") {
        Write-Host "    [Cover Action] Keeping original non-square thumbnail." -ForegroundColor DarkGray
        return $InputImage
    }

    $filter = ""
    $isComplex = $false

    switch ($NonSquareMode) {
        "crop" {
            Write-Host "    [Cover Action] Cropping sides to 1:1 square." -ForegroundColor Cyan
            $filter = "crop='min(iw,ih)':'min(iw,ih)'"
        }
        "blur" {
            Write-Host "    [Cover Action] Formatting on blurred background." -ForegroundColor Cyan
            $filter = "[0:v]scale='max(iw,ih)':'max(iw,ih)',boxblur=40:5[bg];[bg][0:v]overlay=(main_w-overlay_w)/2:(main_h-overlay_h)/2"
            $isComplex = $true
        }
        "stretch" {
            Write-Host "    [Cover Action] Compressing/scaling to 1:1 square." -ForegroundColor Cyan
            $filter = "scale='max(iw,ih)':'max(iw,ih)'"
        }
        default {
            Write-Host "    [Cover Action] Compressing/scaling to 1:1 square (default)." -ForegroundColor Cyan
            $filter = "scale='max(iw,ih)':'max(iw,ih)'"
        }
    }

    if ($isComplex) {
        $args = @("-y", "-i", $InputImage, "-filter_complex", $filter, "-q:v", "2", $OutputImage)
    } else {
        $args = @("-y", "-i", $InputImage, "-vf", $filter, "-q:v", "2", $OutputImage)
    }

    & $FfmpegPath @args 2>&1 | Out-Null

    if ($LASTEXITCODE -eq 0 -and (Test-Path $OutputImage)) {
        return $OutputImage
    }

    Write-Host "    [Cover Warning] Processing failed, using original." -ForegroundColor Yellow
    return $InputImage
}

function Parse-VttLyrics {
    param([string]$VttPath)

    if (-not (Test-Path $VttPath)) { return $null }

    try {
        $lines = Get-Content -LiteralPath $VttPath -Encoding UTF8
        $lyricsLines = [System.Collections.Generic.List[string]]::new()
        $prevLine = ""
        $inStyleBlock = $false

        foreach ($line in $lines) {
            $trimmed = $line.Trim()

            if ($trimmed -match '^STYLE') {
                $inStyleBlock = $true
                continue
            }
            if ($inStyleBlock) {
                if ($trimmed -match '\}') { $inStyleBlock = $false }
                continue
            }
            if ($trimmed -match '^[\}\]\)]') {
                continue
            }

            if ($trimmed -match '^(WEBVTT|NOTE|REGION|\d+$)' -or
                $trimmed -match '^\d{2}:\d{2}' -or
                $trimmed -match '^(Kind:|Language:|##|::cue)' -or
                $trimmed.Length -eq 0) {
                continue
            }

            $clean = [System.Text.RegularExpressions.Regex]::Replace($trimmed, '<[^>]+>', '')
            $clean = [System.Text.RegularExpressions.Regex]::Replace($clean, '[\u200B-\u200D\uFEFF]', '').Trim()

            if ($clean.Length -gt 0 -and $clean -ne $prevLine) {
                $lyricsLines.Add($clean)
                $prevLine = $clean
            }
        }

        if ($lyricsLines.Count -gt 0) {
            return ($lyricsLines -join "`n")
        }
    } catch {}
    return $null
}

function Clean-QueryText {
    param([string]$Text)
    if ([string]::IsNullOrWhiteSpace($Text)) { return "" }
    $cleaned = [System.Text.RegularExpressions.Regex]::Replace($Text, '[\p{Cs}\p{So}\p{Sk}\p{Sm}]', '')
    $cleaned = [System.Text.RegularExpressions.Regex]::Replace($cleaned, '(?i)(【.*?】|\[.*?\]|\(.*?\)|cover|acoustic|official|mv|様|\u69D8)', '')
    return $cleaned.Trim()
}

function Fetch-LyricsFromLrclib {
    param(
        [string]$Title,
        [string]$Artist,
        [string]$Description = ""
    )

    $headers = @{
        "User-Agent" = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
    }

    $cleanTitle = Clean-QueryText -Text $Title
    $cleanArtist = Clean-QueryText -Text $Artist

    $candidatePairs = [System.Collections.Generic.List[PSObject]]::new()

    if (-not [string]::IsNullOrWhiteSpace($Description)) {
        if ($Description -match '(?m)^(?<t>[^/\r\n]+?)\s*[/／\uFF0F]\s*(?<a>[^\r\n]+)') {
            $rawT = Clean-QueryText -Text $Matches['t']
            $rawA = Clean-QueryText -Text $Matches['a']
            if ($rawT.Length -gt 0 -and $rawA.Length -gt 0) {
                $candidatePairs.Add([PSCustomObject]@{ Track = $rawT; Artist = $rawA })
            }
        }
        if ($Description -match '(?m)(?:本家|原曲|Music|作詞|作曲)[:：\uFF1A\s]+(?<a>[^\r\n]+)') {
            $rawA = Clean-QueryText -Text $Matches['a']
            if ($rawA.Length -gt 0) {
                $candidatePairs.Add([PSCustomObject]@{ Track = $cleanTitle; Artist = $rawA })
            }
        }
    }

    if ($Title -match '^(?<t>.+?)\s*[/／\uFF0F-]\s*(?<a>.+)$') {
        $rawT = Clean-QueryText -Text $Matches['t']
        $rawA = Clean-QueryText -Text $Matches['a']
        if ($rawT.Length -gt 0 -and $rawA.Length -gt 0) {
            $candidatePairs.Add([PSCustomObject]@{ Track = $rawT; Artist = $rawA })
            $candidatePairs.Add([PSCustomObject]@{ Track = $rawA; Artist = $rawT })
        }
    }

    $candidatePairs.Add([PSCustomObject]@{ Track = $cleanTitle; Artist = $cleanArtist })
    $candidatePairs.Add([PSCustomObject]@{ Track = $cleanTitle; Artist = $Artist })

    foreach ($pair in $candidatePairs) {
        $t = $pair.Track
        $a = $pair.Artist
        if ([string]::IsNullOrWhiteSpace($t) -or [string]::IsNullOrWhiteSpace($a)) { continue }

        try {
            $encTrack = [System.Uri]::EscapeDataString($t)
            $encArtist = [System.Uri]::EscapeDataString($a)
            $getUrl = "https://lrclib.net/api/get?track_name=$encTrack&artist_name=$encArtist"

            $res = Invoke-RestMethod -Uri $getUrl -Headers $headers -TimeoutSec 4 -ErrorAction Stop
            if ($res) {
                if (-not [string]::IsNullOrWhiteSpace($res.plainLyrics)) {
                    Write-Host "    [Lyrics Match] Hit via '$t' by '$a'" -ForegroundColor DarkCyan
                    return $res.plainLyrics.Trim()
                }
                if (-not [string]::IsNullOrWhiteSpace($res.syncedLyrics)) {
                    $plain = [System.Text.RegularExpressions.Regex]::Replace($res.syncedLyrics, '\[\d{2}:\d{2}\.\d{2,3}\]', '').Trim()
                    if ($plain.Length -gt 0) {
                        Write-Host "    [Lyrics Match] Hit via '$t' by '$a' (synced)" -ForegroundColor DarkCyan
                        return $plain
                    }
                }
            }
        } catch {
        }
    }

    $searchQueries = @(
        "$cleanTitle $cleanArtist",
        $cleanTitle,
        $Title
    ) | Select-Object -Unique | Where-Object { $_.Length -gt 0 }

    foreach ($q in $searchQueries) {
        try {
            $encoded = [System.Uri]::EscapeDataString($q)
            $apiUrl = "https://lrclib.net/api/search?q=$encoded"
            $results = Invoke-RestMethod -Uri $apiUrl -Headers $headers -TimeoutSec 4 -ErrorAction Stop

            if ($results -and $results.Count -gt 0) {
                foreach ($item in $results) {
                    if (-not [string]::IsNullOrWhiteSpace($item.plainLyrics)) {
                        return $item.plainLyrics.Trim()
                    }
                    if (-not [string]::IsNullOrWhiteSpace($item.syncedLyrics)) {
                        $plain = [System.Text.RegularExpressions.Regex]::Replace($item.syncedLyrics, '\[\d{2}:\d{2}\.\d{2,3}\]', '').Trim()
                        if ($plain.Length -gt 0) { return $plain }
                    }
                }
            }
        } catch {}
    }

    return $null
}

function Get-SongLyrics {
    param(
        [string]$SessionDir,
        [string]$Title,
        [string]$Artist,
        [string]$Description = ""
    )

    $subFiles = Get-ChildItem -Path $SessionDir -File |
        Where-Object { $_.Extension -in @(".vtt", ".lrc", ".srt") }

    $targetSub = $null
    if ($subFiles) {
        $targetSub = $subFiles | Where-Object { $_.Name -match '\.ja[^\.]*\.' } | Select-Object -First 1
        if (-not $targetSub) {
            $targetSub = $subFiles | Where-Object { $_.Name -match '\.en[^\.]*\.' } | Select-Object -First 1
        }
        if (-not $targetSub) {
            $targetSub = $subFiles | Select-Object -First 1
        }
    }

    if ($targetSub) {
        $vttLyrics = Parse-VttLyrics -VttPath $targetSub.FullName
        if (-not [string]::IsNullOrWhiteSpace($vttLyrics)) {
            Write-Host "    [Lyrics Source] YouTube subtitle ($($targetSub.Name))" -ForegroundColor Green
            return $vttLyrics
        }
    }

    Write-Host "    [Lyrics Source] Searching online database (LRCLIB)..." -ForegroundColor DarkGray
    $onlineLyrics = Fetch-LyricsFromLrclib -Title $Title -Artist $Artist -Description $Description
    if (-not [string]::IsNullOrWhiteSpace($onlineLyrics)) {
        Write-Host "    [Lyrics Source] Found in online lyrics database!" -ForegroundColor Green
        return $onlineLyrics
    }

    Write-Host "    [Lyrics Source] No lyrics found (skipping)." -ForegroundColor DarkGray
    return $null
}

function Embed-TagsAndArtwork {
    param(
        [string]$AtomicParsleyPath,
        [string]$FfmpegPath,
        [string]$AudioFile,
        [string]$CoverImage,
        [string]$LyricsFile,
        [string]$Title,
        [string]$Artist,
        [string]$CommentUrl,
        [string]$AudioFormat
    )

    if ($AudioFormat -eq "m4a" -and $AtomicParsleyPath -and (Test-Path $AtomicParsleyPath)) {
        $apArgs = @(
            $AudioFile,
            "--title", $Title,
            "--artist", $Artist,
            "--comment", $CommentUrl,
            "--overWrite"
        )
        if ($CoverImage -and (Test-Path $CoverImage)) {
            $apArgs += @("--artwork", $CoverImage)
        }
        if ($LyricsFile -and (Test-Path $LyricsFile)) {
            $apArgs += @("--lyricsFile", $LyricsFile)
        }
        $res = & $AtomicParsleyPath @apArgs 2>&1
        return ($LASTEXITCODE -eq 0)
    }

    $tempTagged = [System.IO.Path]::ChangeExtension($AudioFile, ".tagged.$AudioFormat")
    $ffArgs = @("-y", "-i", $AudioFile)

    if ($CoverImage -and (Test-Path $CoverImage)) {
        $ffArgs += @(
            "-i", $CoverImage,
            "-map", "0:a", "-map", "1:v",
            "-c:a", "copy", "-c:v", "mjpeg",
            "-disposition:v", "attached_pic"
        )
    } else {
        $ffArgs += @("-c", "copy")
    }

    $ffArgs += @(
        "-metadata", "title=$Title",
        "-metadata", "artist=$Artist",
        "-metadata", "comment=$CommentUrl"
    )

    if ($LyricsFile -and (Test-Path $LyricsFile)) {
        $lyrContent = Get-Content -LiteralPath $LyricsFile -Raw -Encoding UTF8
        $ffArgs += @("-metadata", "lyrics=$lyrContent")
    }

    $ffArgs += @($tempTagged)

    & $FfmpegPath @ffArgs 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0 -and (Test-Path $tempTagged)) {
        Move-Item -LiteralPath $tempTagged -Destination $AudioFile -Force
        return $true
    }

    return $false
}

function Main {
    Clear-Host
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "       Universal Music Downloader       " -ForegroundColor White
    Write-Host "    (YouTube / X / TikTok / Niconico)   " -ForegroundColor DarkCyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Config : Format = $($Script:Config['audio_format']), NonSquareCover = $($Script:Config['cover_mode_non_square']), Lyrics = $($Script:Config['embed_lyrics']), Threads = $($Script:Config['concurrent_fragments'])" -ForegroundColor DarkGray
    Write-Host ""

    $tools = Initialize-Tools
    if (-not $tools) { return }

    Write-Host "Checking for yt-dlp updates..." -ForegroundColor Gray
    & $tools.YtDlp -U 2>&1 | Out-Null
    Write-Host "Tools are ready!" -ForegroundColor Green
    Write-Host ""

    $outFolder = if ($Script:Config["output_folder"]) { $Script:Config["output_folder"] } else { "output" }
    $tempFolder = if ($Script:Config["temp_folder"]) { $Script:Config["temp_folder"] } else { "temp_download" }
    $outputDir = Join-Path $PSScriptRoot $outFolder
    $tempBase  = Join-Path $PSScriptRoot $tempFolder
    if (-not (Test-Path $outputDir)) { New-Item -ItemType Directory -Path $outputDir -Force | Out-Null }

    $cookieArgs = Get-CookieArguments

    while ($true) {
        Write-Host "--------------------------------------------------" -ForegroundColor DarkGray
        Write-Host "Enter Media / Music URL (or press Enter to exit):" -ForegroundColor Yellow
        $rawInput = Read-Host "> "

        if ([string]::IsNullOrWhiteSpace($rawInput)) {
            Write-Host "Exiting. Have a great day!" -ForegroundColor Cyan
            break
        }

        $rawTokens = $rawInput -split '\s+' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
        $urls = @()
        foreach ($token in $rawTokens) {
            $normalized = Normalize-Url -RawUrl $token
            if ($normalized -match '^https?://[^\s/$.?#].[^\s]*$') {
                $urls += $normalized
            }
        }

        if ($urls.Count -eq 0) {
            Write-Host "[!] No valid URL detected. Please try again." -ForegroundColor Yellow
            continue
        }

        foreach ($url in $urls) {
            Write-Host "`nTarget: $url" -ForegroundColor Cyan
            $sessionDir = Join-Path $tempBase ([guid]::NewGuid().ToString("N"))
            New-Item -ItemType Directory -Path $sessionDir -Force | Out-Null

            try {
                $audioFmt = $Script:Config["audio_format"]
                $audioQual = $Script:Config["audio_quality"]
                $embedLyr = [bool]$Script:Config["embed_lyrics"]
                $concur = [int]$Script:Config["concurrent_fragments"]

                Write-Host "  [1/5] Downloading audio ($audioFmt), thumbnail & metadata (-N $concur)..." -ForegroundColor White
                $fileTemplate = Join-Path $sessionDir "%(title)s.%(ext)s"

                $siteArgs = Build-SiteSpecificArguments -Url $url -FfmpegPath $tools.Ffmpeg `
                                                       -AudioFormat $audioFmt -AudioQuality $audioQual `
                                                       -ConcurrentFrags $concur -EmbedLyrics $embedLyr
                $dlArgs = $siteArgs + $cookieArgs + @("-o", $fileTemplate, $url)

                & $tools.YtDlp @dlArgs 2>&1 | ForEach-Object {
                    [Console]::WriteLine($_)
                }
                $audioFile = Get-ChildItem -Path $sessionDir -File |
                    Where-Object { $_.Extension -in @(".$audioFmt", ".m4a", ".mp3", ".opus", ".webm", ".flac") } |
                    Select-Object -First 1

                if (-not $audioFile) {
                    Write-Host "  [ERROR] Download failed. Check URL, network, or site access." -ForegroundColor Red
                    continue
                }

                Write-Host "  [2/5] Reading source metadata..." -ForegroundColor White
                $jsonFile = Get-ChildItem -Path $sessionDir -Filter "*.info.json" | Select-Object -First 1
                $finalTitle = [System.IO.Path]::GetFileNameWithoutExtension($audioFile.Name)
                $finalArtist = "Unknown Artist"
                $description = ""

                if ($jsonFile) {
                    try {
                        $meta = Get-Content $jsonFile.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
                        if ($meta.title) { $finalTitle = $meta.title }
                        if ($meta.description) { $description = $meta.description }
                        if ($meta.artist) {
                            $finalArtist = $meta.artist
                        } elseif ($meta.uploader) {
                            $finalArtist = $meta.uploader
                        } elseif ($meta.channel) {
                            $finalArtist = $meta.channel
                        }
                    } catch {}
                }

                Write-Host "    Title  : $finalTitle" -ForegroundColor Green
                Write-Host "    Artist : $finalArtist" -ForegroundColor Green

                Write-Host "  [3/5] Checking and formatting cover art..." -ForegroundColor White
                $thumbFile = Get-ChildItem -Path $sessionDir -File |
                    Where-Object { $_.Extension -in @(".jpg", ".jpeg", ".png", ".webp") } |
                    Select-Object -First 1

                $processedCover = $null
                if ($thumbFile) {
                    $coverOut = Join-Path $sessionDir "cover_square.jpg"
                    $nonSquareMode = $Script:Config["cover_mode_non_square"]
                    $autoCrop = if ($null -ne $Script:Config["auto_crop_shorts"]) { [bool]$Script:Config["auto_crop_shorts"] } else { $true }
                    if ($url -match "youtube\.com/shorts/" -and $autoCrop) {
                        Write-Host "    [Shorts] YouTube Shorts detected: Automatically applying 'crop' mode." -ForegroundColor Cyan
                        $nonSquareMode = "crop"
                    }

                    $processedCover = Format-CoverArt -FfmpegPath $tools.Ffmpeg `
                                                      -FfprobePath $tools.Ffprobe `
                                                      -InputImage $thumbFile.FullName `
                                                      -OutputImage $coverOut `
                                                      -NonSquareMode $nonSquareMode
                } else {
                    Write-Host "    No thumbnail found; skipping cover art." -ForegroundColor Yellow
                }

                $lyricsFile = $null
                if ($embedLyr) {
                    Write-Host "  [4/5] Fetching lyrics..." -ForegroundColor White
                    $lyricsText = Get-SongLyrics -SessionDir $sessionDir -Title $finalTitle -Artist $finalArtist -Description $description
                    if (-not [string]::IsNullOrWhiteSpace($lyricsText)) {
                        $lyricsFile = Join-Path $sessionDir "lyrics.txt"
                        Set-Content -Path $lyricsFile -Value $lyricsText -Encoding UTF8
                    }
                } else {
                    Write-Host "  [4/5] Lyrics embedding disabled in config." -ForegroundColor DarkGray
                }

                Write-Host "  [5/5] Tagging audio and embedding cover & lyrics..." -ForegroundColor White

                $targetExt = if ($audioFile.Extension) { $audioFile.Extension } else { ".$audioFmt" }
                $sanitizedTitle = $finalTitle
                $invalidChars = [System.IO.Path]::GetInvalidFileNameChars()
                foreach ($c in $invalidChars) {
                    $sanitizedTitle = $sanitizedTitle.Replace($c, "_")
                }
                $cleanFileName = "$sanitizedTitle$targetExt"
                $finalDestination = Join-Path $outputDir $cleanFileName

                $counter = 1
                $baseNameWithoutExt = [System.IO.Path]::GetFileNameWithoutExtension($cleanFileName)
                while (Test-Path $finalDestination) {
                    $finalDestination = Join-Path $outputDir "$baseNameWithoutExt ($counter)$targetExt"
                    $counter++
                }

                $tagged = Embed-TagsAndArtwork -AtomicParsleyPath $tools.AtomicParsley `
                                               -FfmpegPath $tools.Ffmpeg `
                                               -AudioFile $audioFile.FullName `
                                               -CoverImage $processedCover `
                                               -LyricsFile $lyricsFile `
                                               -Title $finalTitle `
                                               -Artist $finalArtist `
                                               -CommentUrl $url `
                                               -AudioFormat $audioFmt

                if ($tagged) {
                    Move-Item -LiteralPath $audioFile.FullName -Destination $finalDestination -Force
                    Write-Host "  [SUCCESS] Saved: $finalDestination" -ForegroundColor Green
                } else {
                    Write-Host "  [!] Tagging had warnings, saving audio file." -ForegroundColor Yellow
                    Move-Item -LiteralPath $audioFile.FullName -Destination $finalDestination -Force
                    Write-Host "  [SAVED] Saved: $finalDestination" -ForegroundColor Green
                }

            } catch {
                Write-Host "  [ERROR] An unexpected error occurred: $($_.Exception.Message)" -ForegroundColor Red
            } finally {
                if (Test-Path $sessionDir) {
                    Remove-Item -LiteralPath $sessionDir -Recurse -Force -ErrorAction SilentlyContinue
                }
            }
        }
    }

    if (Test-Path $tempBase) {
        Remove-Item -LiteralPath $tempBase -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Main
