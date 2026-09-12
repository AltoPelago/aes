[CmdletBinding()]
param(
    [ValidateRange(1, 1000)]
    [int]$IncrementalIterations = 15,

    [string]$OutputDirectory = ''
)

$ErrorActionPreference = 'Stop'
$bundleDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $bundleDirectory

if (-not [Environment]::Is64BitProcess) {
    throw 'The AES benchmark bundle requires 64-bit Windows on an x64 processor.'
}

$architecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
if ($architecture -ne 'X64') {
    throw "Expected native X64 execution, received $architecture."
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $OutputDirectory = Join-Path $bundleDirectory "results-$stamp"
} elseif (-not [IO.Path]::IsPathRooted($OutputDirectory)) {
    $OutputDirectory = Join-Path $bundleDirectory $OutputDirectory
}
if (Test-Path $OutputDirectory) {
    throw "Output directory already exists: $OutputDirectory"
}
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null

function Assert-BundleHashes {
    $manifest = Join-Path $bundleDirectory 'SHA256SUMS.txt'
    foreach ($line in Get-Content $manifest) {
        if ($line -notmatch '^([0-9a-f]{64})  (.+)$') {
            throw "Invalid checksum entry: $line"
        }
        $expected = $Matches[1]
        $name = $Matches[2]
        $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $bundleDirectory $name)).Hash.ToLowerInvariant()
        if ($actual -ne $expected) {
            throw "Checksum mismatch for $name"
        }
    }
}

function Invoke-Benchmark {
    param(
        [string]$Name,
        [string]$Executable,
        [string[]]$Arguments = @()
    )

    $output = Join-Path $OutputDirectory "$Name.txt"
    Write-Host "Running $Name..."
    & (Join-Path $bundleDirectory $Executable) @Arguments 2>&1 |
        Tee-Object -FilePath $output
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

Assert-BundleHashes

$processor = Get-CimInstance Win32_Processor | Select-Object -First 1
$operatingSystem = Get-CimInstance Win32_OperatingSystem
$system = [ordered]@{
    collected_at = (Get-Date).ToUniversalTime().ToString('o')
    machine = $env:COMPUTERNAME
    os = $operatingSystem.Caption
    os_version = $operatingSystem.Version
    process_architecture = $architecture
    processor = $processor.Name.Trim()
    physical_cores = $processor.NumberOfCores
    logical_processors = $processor.NumberOfLogicalProcessors
    incremental_iterations = $IncrementalIterations
    power_source = if ((Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue).BatteryStatus -eq 2) {
        'ac-power'
    } else {
        'unknown-or-battery'
    }
    build = @(Get-Content (Join-Path $bundleDirectory 'BUILD.txt'))
}
$system | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 (Join-Path $OutputDirectory 'system.json')

Invoke-Benchmark -Name 'scalar-codec-phases' -Executable 'bench_codec_phases.exe'
Invoke-Benchmark -Name 'document-pipeline' -Executable 'bench_telex_pipeline.exe'

$previousIterations = $env:AES_PIPELINE_BENCH_ITERATIONS
$env:AES_PIPELINE_BENCH_ITERATIONS = [string]$IncrementalIterations
try {
    Invoke-Benchmark `
        -Name 'within-document-pipeline' `
        -Executable 'aes_telex_incremental_bench.exe' `
        -Arguments @(
            'telex_incremental::tests::benchmark_capacity_one_within_document_pipeline',
            '--exact',
            '--ignored',
            '--nocapture',
            '--test-threads=1'
        )
} finally {
    $env:AES_PIPELINE_BENCH_ITERATIONS = $previousIterations
}

$archive = "$OutputDirectory.zip"
if (Test-Path $archive) {
    throw "Result archive already exists: $archive"
}
Compress-Archive -Path (Join-Path $OutputDirectory '*') -DestinationPath $archive

Write-Host ''
Write-Host 'Benchmarks completed successfully.'
Write-Host "Results directory: $OutputDirectory"
Write-Host "Shareable archive:  $archive"
