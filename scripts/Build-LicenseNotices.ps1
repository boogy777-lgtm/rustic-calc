[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Win32', 'Slint', 'GPUI')]
    [string] $Edition,
    [Parameter(Mandatory = $true)]
    [string] $OutputPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$target = 'x86_64-pc-windows-msvc'
$manifests = @{
    Win32 = 'Cargo.toml'
    Slint = 'editions\slint\Cargo.toml'
    GPUI = 'editions\gpui\Cargo.toml'
}
$manifestPath = Join-Path $root $manifests[$Edition]
$outputPath = [IO.Path]::GetFullPath($OutputPath)
$outputDirectory = Split-Path -Parent $outputPath

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Cargo manifest for $Edition was not found: $manifestPath"
}
[IO.Directory]::CreateDirectory($outputDirectory) | Out-Null

$exceptionTablePath = Join-Path $PSScriptRoot 'License-Notice-Exceptions.psd1'
if (-not (Test-Path -LiteralPath $exceptionTablePath -PathType Leaf)) {
    throw "The pinned license exception table is missing: $exceptionTablePath"
}
$licenseExceptions = Import-PowerShellDataFile -LiteralPath $exceptionTablePath

# Cargo metadata supplies locked package/license fields. Cargo tree is the
# feature-resolved source of truth for active target edges; dev-only and inactive
# optional package records in metadata are deliberately ignored.
$metadataLines = & cargo metadata --offline --locked --format-version 1 `
    --filter-platform $target --manifest-path $manifestPath
if ($LASTEXITCODE -ne 0) {
    throw "Offline locked Cargo metadata failed for $Edition."
}
$metadata = ($metadataLines -join "`n") | ConvertFrom-Json
$packagesByIdentity = @{}
foreach ($package in $metadata.packages) {
    if ($null -eq $package.source) { continue }
    $key = [string]$package.name + '|' + [string]$package.version + '|' + [string]$package.repository
    if (-not $packagesByIdentity.ContainsKey($key)) {
        $packagesByIdentity[$key] = [Collections.Generic.List[object]]::new()
    }
    $packagesByIdentity[$key].Add($package)
}

function Get-CargoTreePackageIds([string] $Edges) {
    $lines = & cargo tree --offline --locked --target $target --manifest-path $manifestPath `
        --edges $Edges --prefix none --format '{p}|{r}'
    if ($LASTEXITCODE -ne 0) {
        throw "Offline locked Cargo tree failed for $Edition ($Edges edges)."
    }

    $ids = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($line in $lines) {
        if ([string]::IsNullOrWhiteSpace([string]$line)) { continue }
        $text = ([string]$line).Trim() -replace '\s+\(\*\)$', ''
        if ($text.StartsWith('"') -and $text.EndsWith('"')) {
            $text = $text.Substring(1, $text.Length - 2)
        }
        $separator = $text.IndexOf('|')
        if ($separator -lt 0) {
            throw "Cannot map cargo tree output to package metadata for $($Edition): $line"
        }
        $packageText = $text.Substring(0, $separator)
        $repository = $text.Substring($separator + 1)
        $match = [regex]::Match($packageText, '^\s*(?<name>[A-Za-z0-9][A-Za-z0-9_.-]*) v(?<version>[^\s]+)')
        if (-not $match.Success) {
            throw "Cannot map cargo tree output to package metadata for $($Edition): $line"
        }
        $key = $match.Groups['name'].Value + '|' + $match.Groups['version'].Value + '|' + $repository
        if (-not $packagesByIdentity.ContainsKey($key)) {
            # Local path packages are covered by the repository LICENSE/NOTICE.
            continue
        }
        $matches = $packagesByIdentity[$key]
        if ($matches.Count -ne 1) {
            throw "Cargo tree package identity is ambiguous in metadata (name/version/repository): $key"
        }
        [void]$ids.Add([string]$matches[0].id)
    }
    return ,$ids
}

$normalIds = Get-CargoTreePackageIds 'normal'
$normalAndBuildIds = Get-CargoTreePackageIds 'normal,build'
$orderedPackages = [Collections.Generic.SortedDictionary[string, object]]::new([StringComparer]::Ordinal)
foreach ($id in $normalAndBuildIds) {
    $package = $metadata.packages | Where-Object { [string]$_.id -ceq $id } | Select-Object -First 1
    if ($null -eq $package) { throw "Cargo metadata omitted tree package data: $id" }
    $graph = if ($normalIds.Contains($id)) {
        'normal target dependency graph'
    } else {
        'build-only/codegen dependency closure'
    }
    $key = [string]$package.name + '|' + [string]$package.version + '|' + [string]$package.id
    $orderedPackages[$key] = [pscustomobject]@{ Package = $package; Graph = $graph }
}
$packages = @($orderedPackages.Values)

function Get-PackageLegalFiles([object] $Package) {
    $packageRoot = [IO.Path]::GetFullPath((Split-Path -Parent ([string]$Package.manifest_path)))
    $packageRootPrefix = $packageRoot.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    $files = [Collections.Generic.SortedDictionary[string, string]]::new([StringComparer]::OrdinalIgnoreCase)

    if (-not [string]::IsNullOrWhiteSpace([string]$Package.license_file)) {
        $declaredPath = [string]$Package.license_file
        if ([IO.Path]::IsPathRooted($declaredPath)) {
            $fullPath = [IO.Path]::GetFullPath($declaredPath)
        } else {
            $fullPath = [IO.Path]::GetFullPath((Join-Path $packageRoot $declaredPath))
        }
        if (-not $fullPath.StartsWith($packageRootPrefix, [StringComparison]::OrdinalIgnoreCase) -or
            -not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
            throw "$($Package.name) $($Package.version): declared license_file is missing or outside its package."
        }
        $relative = $fullPath.Substring($packageRootPrefix.Length).Replace('\', '/')
        $files[$relative] = $fullPath
    }

    $patterns = @(
        'LICENSE*', 'LICENCE*', 'COPYING*', 'NOTICE*',
        'COPYRIGHT*', 'AUTHORS*', 'PATENTS*'
    )
    $searchRoot = Join-Path $packageRoot '*'
    foreach ($file in Get-ChildItem -Path $searchRoot -File -Recurse -Include $patterns -Force) {
        $relative = $file.FullName.Substring($packageRootPrefix.Length).Replace('\', '/')
        $isLegalFile = $relative -match '(^|/)(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|AUTHORS|PATENTS)(?:$|[._ -])'
        if ($isLegalFile) {
            $files[$relative] = $file.FullName
        }
    }
    foreach ($directory in Get-ChildItem -LiteralPath $packageRoot -Directory -Recurse -Force |
        Where-Object { $_.Name -ceq 'LICENSES' }) {
        foreach ($file in Get-ChildItem -LiteralPath $directory.FullName -File -Recurse -Force) {
            $relative = $file.FullName.Substring($packageRootPrefix.Length).Replace('\', '/')
            $files[$relative] = $file.FullName
        }
    }
    return @($files.GetEnumerator() | ForEach-Object {
        [pscustomobject]@{
            RelativePath = $_.Key
            FullPath = $_.Value
            SourceUrl = ''
            SourceKind = 'locked crate archive'
            ProvenanceNote = ''
            Sha256 = ''
        }
    })
}

function Get-NormalizedTextSha256([string] $Text) {
    $normalized = $Text.Replace("`r`n", "`n").Replace("`r", "`n")
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes($normalized)
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($algorithm.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally {
        $algorithm.Dispose()
    }
}

$missing = [Collections.Generic.List[string]]::new()
$packageLegalFiles = @{}
foreach ($entry in $packages) {
    $package = $entry.Package
    $exceptionKey = [string]$package.name + '|' + [string]$package.version
    if ([string]::IsNullOrWhiteSpace([string]$package.license) -and
        [string]::IsNullOrWhiteSpace([string]$package.license_file)) {
        $missing.Add("$($package.name) $($package.version) [$($entry.Graph)]: Cargo metadata has no license or license_file.")
        continue
    }
    try {
        $legalFiles = [Collections.Generic.List[object]]::new()
        foreach ($file in @(Get-PackageLegalFiles $package)) {
            $text = [IO.File]::ReadAllText($file.FullPath)
            $file.Sha256 = Get-NormalizedTextSha256 $text
            [void]$legalFiles.Add($file)
        }
        if ($licenseExceptions.Packages.ContainsKey($exceptionKey)) {
            foreach ($textId in $licenseExceptions.Packages[$exceptionKey].Files) {
                if (-not $licenseExceptions.LegalTexts.ContainsKey([string]$textId)) {
                    throw "the exception table references an unknown legal text id: $textId"
                }
                $override = $licenseExceptions.LegalTexts[[string]$textId]
                $overridePath = [IO.Path]::GetFullPath((Join-Path $root ([string]$override.Path)))
                $rootPrefix = [IO.Path]::GetFullPath($root).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
                if (-not $overridePath.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase) -or
                    -not (Test-Path -LiteralPath $overridePath -PathType Leaf)) {
                    throw "pinned legal text is missing or outside the repository: $($override.Path)"
                }
                $overrideText = [IO.File]::ReadAllText($overridePath)
                $actualHash = Get-NormalizedTextSha256 $overrideText
                if ([string]::IsNullOrWhiteSpace($overrideText) -or
                    $actualHash -cne [string]$override.Sha256) {
                    throw "pinned legal text digest mismatch for $($override.Path)"
                }
                [void]$legalFiles.Add([pscustomobject]@{
                    RelativePath = [string]$override.Label
                    FullPath = $overridePath
                    SourceUrl = [string]$override.SourceUrl
                    SourceKind = [string]$override.SourceKind
                    ProvenanceNote = [string]$override.ProvenanceNote
                    Sha256 = $actualHash
                })
            }
        }
    } catch {
        $missing.Add("$($package.name) $($package.version) [$($entry.Graph)]: $($_.Exception.Message)")
        continue
    }
    if ($legalFiles.Count -eq 0) {
        $missing.Add("$($package.name) $($package.version) [$($entry.Graph)]: no recognized upstream license/NOTICE/copyright text file and no exact-version exception.")
        continue
    }
    foreach ($legalFile in $legalFiles) {
        if ([string]::IsNullOrWhiteSpace([IO.File]::ReadAllText($legalFile.FullPath))) {
            $missing.Add("$($package.name) $($package.version) [$($entry.Graph)]: empty legal text at $($legalFile.RelativePath).")
        }
    }
    if ($legalFiles.Count -gt 0) {
        $packageLegalFiles[[string]$package.id] = @($legalFiles)
    }
}

if ($missing.Count -gt 0) {
    Write-Error ("Refusing to create an incomplete $Edition notice bundle:`n - " + ($missing -join "`n - "))
    throw 'Resolve every reported upstream license/NOTICE text gap before packaging.'
}

$projectFiles = @(
    [pscustomobject]@{ RelativePath = 'LICENSE'; FullPath = (Join-Path $root 'LICENSE') },
    [pscustomobject]@{ RelativePath = 'NOTICE.md'; FullPath = (Join-Path $root 'NOTICE.md') },
    [pscustomobject]@{ RelativePath = 'third_party/geo-ts-msk-LICENSE.txt'; FullPath = (Join-Path $root 'third_party\geo-ts-msk-LICENSE.txt') }
)
foreach ($file in $projectFiles) {
    if (-not (Test-Path -LiteralPath $file.FullPath -PathType Leaf)) {
        throw "Required project notice input is missing: $($file.RelativePath)"
    }
}

$builder = [Text.StringBuilder]::new()
function Add-NoticeLine([string] $Text) {
    [void]$script:builder.Append($Text)
    [void]$script:builder.Append("`n")
}
function Add-NoticeText([string] $Text) {
    $normalized = $Text.Replace("`r`n", "`n").Replace("`r", "`n").TrimEnd("`n")
    [void]$script:builder.Append($normalized)
    [void]$script:builder.Append("`n")
}

Add-NoticeLine 'Rustic Calc - Third-Party License and Notice Bundle'
Add-NoticeLine "Edition: $Edition"
Add-NoticeLine "Target: $target"
Add-NoticeLine 'Normal target graph: cargo tree --offline --locked --target x86_64-pc-windows-msvc --edges normal --prefix none.'
Add-NoticeLine 'Build/codegen closure: additional packages from cargo tree --offline --locked --target x86_64-pc-windows-msvc --edges normal,build --prefix none.'
Add-NoticeLine 'Dev-only and inactive optional packages are excluded from both graphs.'
Add-NoticeLine 'Normal graph includes active Cargo normal dependencies, including proc-macro dependencies; it is not asserted to be a literal linked-runtime list.'
Add-NoticeLine 'Cargo tree package identities map to locked metadata by exact name, version, and repository; ambiguous mappings fail.'
Add-NoticeLine "Registry/git packages: $($packages.Count); normal graph: $($normalIds.Count); build-only/codegen closure: $($packages.Count - $normalIds.Count)"
Add-NoticeLine 'Collection policy: packaging fails if a package lacks license metadata or a recognized legal text file.'
Add-NoticeLine 'Pinned exceptions retain their source and attribution and use SHA-256 checks. Non-release sources and license selections are explicitly identified.'
Add-NoticeLine 'This inventory preserves Cargo license expressions and available upstream texts; it is not a legal determination.'
Add-NoticeLine ''
Add-NoticeLine 'PROJECT AND BUNDLED-DATA NOTICES'
foreach ($file in $projectFiles) {
    Add-NoticeLine ''
    Add-NoticeLine "----- BEGIN $($file.RelativePath) -----"
    Add-NoticeText ([IO.File]::ReadAllText($file.FullPath))
    Add-NoticeLine "----- END $($file.RelativePath) -----"
}

foreach ($entry in $packages) {
    $package = $entry.Package
    $expression = if ([string]::IsNullOrWhiteSpace([string]$package.license)) {
        "(license-file: $($package.license_file); no SPDX expression in Cargo metadata)"
    } else {
        [string]$package.license
    }
    Add-NoticeLine ''
    Add-NoticeLine ('=' * 78)
    Add-NoticeLine "Package: $($package.name) $($package.version)"
    Add-NoticeLine "Graph: $($entry.Graph)"
    Add-NoticeLine "Cargo license: $expression"
    Add-NoticeLine "Source: $($package.source)"
    if (-not [string]::IsNullOrWhiteSpace([string]$package.repository)) {
        Add-NoticeLine "Upstream repository: $($package.repository)"
    }
    if (@($package.authors).Count -gt 0) {
        Add-NoticeLine "Cargo authors: $(@($package.authors) -join '; ')"
    }

    foreach ($file in $packageLegalFiles[[string]$package.id]) {
        Add-NoticeLine ''
        Add-NoticeLine "----- BEGIN $($package.name) $($package.version) / $($file.RelativePath) -----"
        Add-NoticeLine "Legal text provenance: $($file.SourceKind)"
        if (-not [string]::IsNullOrWhiteSpace([string]$file.SourceUrl)) {
            Add-NoticeLine "Legal text URL: $($file.SourceUrl)"
        }
        if (-not [string]::IsNullOrWhiteSpace([string]$file.ProvenanceNote)) {
            Add-NoticeLine "Provenance note: $($file.ProvenanceNote)"
        }
        Add-NoticeLine "LF-normalized UTF-8 SHA-256: $($file.Sha256)"
        Add-NoticeLine ''
        Add-NoticeText ([IO.File]::ReadAllText($file.FullPath))
        Add-NoticeLine "----- END $($package.name) $($package.version) / $($file.RelativePath) -----"
    }
}

[IO.File]::WriteAllText($outputPath, $builder.ToString(), [Text.UTF8Encoding]::new($false))
Write-Host "Created $Edition notice bundle: $outputPath ($($packages.Count) registry/git packages)"
