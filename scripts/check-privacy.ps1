[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Add-Failure {
    param(
        [Parameter(Mandatory = $true)]
        [AllowEmptyCollection()]
        [System.Collections.Generic.List[string]]$Failures,
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    $Failures.Add($Message)
}

function Test-TextCandidate {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $textExtensions = @(
        ".bat", ".cmd", ".conf", ".css", ".csv", ".gitignore", ".htm", ".html",
        ".ini", ".js", ".json", ".jsonc", ".lock", ".md", ".mjs", ".ps1",
        ".rs", ".scss", ".sh", ".sql", ".svg", ".toml", ".ts", ".tsx",
        ".txt", ".vue", ".xml", ".yaml", ".yml"
    )
    $extension = [System.IO.Path]::GetExtension($Path).ToLowerInvariant()
    $leafName = [System.IO.Path]::GetFileName($Path)

    return ($textExtensions -contains $extension) -or
        ($leafName -in @(".gitignore", "Dockerfile", "LICENSE", "Makefile")) -or
        $leafName.StartsWith(".env", [System.StringComparison]::OrdinalIgnoreCase)
}

$repoRootOutput = & git rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0 -or -not $repoRootOutput) {
    throw "Cannot determine the Git repository root. Run this script inside the repository."
}

$repoRoot = [System.IO.Path]::GetFullPath(($repoRootOutput | Select-Object -First 1))
Push-Location -LiteralPath $repoRoot

try {
    $failures = [System.Collections.Generic.List[string]]::new()

    Write-Host "[privacy] Checking .gitignore coverage for local-only data..."
    $requiredIgnoredSamples = [ordered]@{
        ".env"                                      = "environment file"
        "custom-runtime/.env.local"                 = "nested environment file"
        "custom-runtime/config.json"                = "nested AI configuration"
        "custom-runtime/apps.json"                  = "nested launcher data"
        "custom-runtime/storage.json"               = "nested path and usage data"
        "custom-runtime/pet-memory.json"            = "pet memory"
        "custom-runtime/chat-attachments/placeholder" = "local chat attachment"
        "custom-runtime/music-library.json"         = "local music library"
        "custom-runtime/music-platforms/session.json" = "music platform session"
        "custom-runtime/netease-cloud-session.json" = "Netease Cloud session"
        "custom-runtime/kugou-session.json"         = "Kugou session"
        "custom-runtime/client.key"                 = "private key"
        "custom-runtime/client.pem"                 = "certificate or private key"
        "src-tauri/target/release/private-artifact" = "Rust build cache"
        "dist/private-artifact.js"                  = "frontend build output"
    }

    foreach ($sample in $requiredIgnoredSamples.Keys) {
        & git check-ignore --no-index --quiet -- $sample
        if ($LASTEXITCODE -ne 0) {
            Add-Failure -Failures $failures -Message (".gitignore does not cover {0} (sample: {1})" -f $requiredIgnoredSamples[$sample], $sample)
        }
    }

    Write-Host "[privacy] Checking candidate file names..."
    $candidateFiles = @(& git -c core.quotepath=false ls-files --cached --others --exclude-standard)
    if ($LASTEXITCODE -ne 0) {
        throw "Cannot read the Git file list."
    }

    $forbiddenPathPatterns = @(
        [pscustomobject]@{ Name = "environment file"; Pattern = '(?i)(^|/)\.env(?:$|\.)' },
        [pscustomobject]@{ Name = "local configuration or usage data"; Pattern = '(?i)(^|/)(?:config|apps|storage|music-library)\.json$' },
        [pscustomobject]@{ Name = "pet memory database"; Pattern = '(?i)(^|/)pet-memory\.db(?:-[^/]*)?$' },
        [pscustomobject]@{ Name = "local chat attachment"; Pattern = '(?i)(^|/)chat-attachments/' },
        [pscustomobject]@{ Name = "pet memory, story save or chat export"; Pattern = '(?i)(^|/)(?:pet-memory(?:[^/]*)|memory-export(?:[^/]*)|story-saves(?:[^/]*)|memory|memories|chat-history)\.json$' },
        [pscustomobject]@{ Name = "music platform session"; Pattern = '(?i)(^|/)music-platforms/' },
        [pscustomobject]@{ Name = "music platform session"; Pattern = '(?i)(^|/)(?:netease-cloud|kugou)[^/]*\.json$' },
        [pscustomobject]@{ Name = "certificate, key or credential file"; Pattern = '(?i)\.(?:key|pem|p12|pfx|crt|token|secrets?|credentials?)$' },
        [pscustomobject]@{ Name = "local application data directory"; Pattern = '(?i)^(?:PetDrawer|app-data|data|user-data|local-data)/' },
        [pscustomobject]@{ Name = "build or package output"; Pattern = '(?i)^(?:dist|target|src-tauri/target)/' }
    )

    foreach ($candidateFile in $candidateFiles) {
        if ([string]::IsNullOrWhiteSpace($candidateFile)) {
            continue
        }

        $normalizedPath = $candidateFile.Replace("\", "/")
        if ([System.IO.Path]::GetFileName($normalizedPath) -ieq ".env.example") {
            continue
        }

        foreach ($pathPattern in $forbiddenPathPatterns) {
            if ($normalizedPath -match $pathPattern.Pattern) {
                Add-Failure -Failures $failures -Message ("Forbidden {0}: {1}" -f $pathPattern.Name, $normalizedPath)
                break
            }
        }
    }

    Write-Host "[privacy] Scanning strong secret, credential and local-user-path signatures (matched text is never printed)..."
    $secretPatterns = @(
        [pscustomobject]@{ Name = "OpenAI-style key"; Pattern = '(?<![A-Za-z0-9])sk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{20,}' },
        [pscustomobject]@{ Name = "Google API key"; Pattern = '(?<![A-Za-z0-9])AIza[0-9A-Za-z_-]{30,}' },
        [pscustomobject]@{ Name = "GitHub token"; Pattern = '(?<![A-Za-z0-9])gh[pousr]_[A-Za-z0-9]{30,}' },
        [pscustomobject]@{ Name = "AWS access key"; Pattern = '(?<![A-Z0-9])(?:AKIA|ASIA)[0-9A-Z]{16}(?![A-Z0-9])' },
        [pscustomobject]@{ Name = "Slack token"; Pattern = '(?<![A-Za-z0-9])xox[baprs]-[0-9A-Za-z-]{20,}' },
        [pscustomobject]@{ Name = "private key body"; Pattern = ('-----BEGIN ' + '(?:RSA |EC |OPENSSH )?PRIVATE KEY-----') },
        [pscustomobject]@{ Name = "hard-coded high-entropy credential"; Pattern = '(?i)(?:api[_-]?key|access[_-]?token|client[_-]?secret|credential|password)\s*[:=]\s*["''][A-Za-z0-9+/_=-]{24,}["'']' },
        [pscustomobject]@{ Name = "URL-embedded account password"; Pattern = '(?i)\b(?:https?|postgres(?:ql)?|mysql)://[^/\s:@]+:[^@\s/]{8,}@' },
        [pscustomobject]@{ Name = "absolute local user path"; Pattern = '(?i)(?:[A-Z]:\\Users\\[^\\/:*?"<>|\r\n]+\\|/(?:Users|home)/[^/\s"'']+/)' }
    )

    foreach ($candidateFile in $candidateFiles) {
        if ([string]::IsNullOrWhiteSpace($candidateFile) -or -not (Test-TextCandidate -Path $candidateFile)) {
            continue
        }

        $fullPath = Join-Path -Path $repoRoot -ChildPath $candidateFile
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            continue
        }

        $lineNumber = 0
        try {
            foreach ($line in [System.IO.File]::ReadLines($fullPath)) {
                $lineNumber++
                foreach ($secretPattern in $secretPatterns) {
                    if ([regex]::IsMatch($line, $secretPattern.Pattern)) {
                        Add-Failure -Failures $failures -Message ("Possible {0}: {1}:{2}" -f $secretPattern.Name, $candidateFile, $lineNumber)
                    }
                }
            }
        }
        catch {
            Add-Failure -Failures $failures -Message ("Cannot safely scan text file: {0}" -f $candidateFile)
        }
    }

    if ($failures.Count -gt 0) {
        Write-Host "[privacy] FAILED with $($failures.Count) issue(s):" -ForegroundColor Red
        foreach ($failure in $failures) {
            Write-Host ("  - {0}" -f $failure) -ForegroundColor Red
        }
        Write-Host "[privacy] The report only includes the issue type, repository-relative path and line number." -ForegroundColor Yellow
        exit 1
    }

    Write-Host "[privacy] PASSED: no local-only privacy files or strong secret signatures found." -ForegroundColor Green
}
finally {
    Pop-Location
}
