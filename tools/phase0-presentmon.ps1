param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("clear", "edit")]
    [string]$Kind,
    [Parameter(Mandatory = $true)]
    [string]$RunId,
    [Parameter(Mandatory = $true)]
    [string]$Output,
    [Parameter(Mandatory = $true)]
    [string]$NvideExe,
    [Parameter(Mandatory = $true)]
    [string]$PresentMonExe,
    [int]$WarmupSeconds = 10,
    [int]$MeasureSeconds = 30,
    [int]$WarmupEdits = 10,
    [int]$MeasureEdits = 30,
    [switch]$UnboundDiagnostic
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class Phase0Native {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct STARTUPINFO {
        public uint cb;
        public string lpReserved;
        public string lpDesktop;
        public string lpTitle;
        public uint dwX;
        public uint dwY;
        public uint dwXSize;
        public uint dwYSize;
        public uint dwXCountChars;
        public uint dwYCountChars;
        public uint dwFillAttribute;
        public uint dwFlags;
        public ushort wShowWindow;
        public ushort cbReserved2;
        public IntPtr lpReserved2;
        public IntPtr hStdInput;
        public IntPtr hStdOutput;
        public IntPtr hStdError;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct PROCESS_INFORMATION {
        public IntPtr hProcess;
        public IntPtr hThread;
        public uint dwProcessId;
        public uint dwThreadId;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool CreateProcessW(
        string applicationName,
        StringBuilder commandLine,
        IntPtr processAttributes,
        IntPtr threadAttributes,
        bool inheritHandles,
        uint creationFlags,
        IntPtr environment,
        string currentDirectory,
        ref STARTUPINFO startupInfo,
        out PROCESS_INFORMATION processInformation);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern uint ResumeThread(IntPtr thread);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool CloseHandle(IntPtr handle);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool MoveFileExW(string existingName, string newName, uint flags);
}
"@

function Quote-Argument([string]$Value) {
    return '"' + $Value.Replace('"', '\"') + '"'
}

function Parse-InvariantDouble([string]$Value) {
    return [double]::Parse($Value, [Globalization.CultureInfo]::InvariantCulture)
}

if (!(Test-Path -LiteralPath $NvideExe -PathType Leaf)) {
    throw "NVide executable not found: $NvideExe"
}
if (!(Test-Path -LiteralPath $PresentMonExe -PathType Leaf)) {
    throw "PresentMon executable not found: $PresentMonExe"
}
$repoRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $NvideExe))
$git = (Get-Command git.exe -ErrorAction Stop).Source
$gitCommit = (& $git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $gitCommit -notmatch '^[0-9a-f]{40}$') {
    throw "cannot resolve the benchmark commit"
}
$trackedChanges = & $git -C $repoRoot status --porcelain --untracked-files=no
if ($LASTEXITCODE -ne 0 -or $trackedChanges) {
    throw "the benchmark checkout has tracked changes"
}

New-Item -ItemType Directory -Force -Path $Output | Out-Null
$rawPath = Join-Path $Output "presentmon.csv"
$consolePath = Join-Path $Output "presentmon-stderr.txt"
$captureManifestPath = Join-Path $Output "capture-manifest.txt"
$ackPath = Join-Path $Output "displayed-ack.csv"
$session = "nvide-$($RunId -replace '[^A-Za-z0-9]', '-')"

$appArguments = if ($Kind -eq "clear") {
    @(
        "--phase0-benchmark", "clear",
        "--run-id", $RunId,
        "--output", $Output,
        "--warmup-seconds", $WarmupSeconds,
        "--measure-seconds", $MeasureSeconds
    )
} else {
    @(
        "--phase0-benchmark", "edit",
        "--run-id", $RunId,
        "--output", $Output,
        "--warmup-edits", $WarmupEdits,
        "--measure-edits", $MeasureEdits
    )
}
if ($UnboundDiagnostic) {
    $appArguments += "--unbound-diagnostic"
}

$commandLine = New-Object Text.StringBuilder
[void]$commandLine.Append((Quote-Argument $NvideExe))
foreach ($argument in $appArguments) {
    [void]$commandLine.Append(" ")
    [void]$commandLine.Append((Quote-Argument ([string]$argument)))
}

$startup = New-Object Phase0Native+STARTUPINFO
$startup.cb = [Runtime.InteropServices.Marshal]::SizeOf($startup)
$processInfo = New-Object Phase0Native+PROCESS_INFORMATION
$created = [Phase0Native]::CreateProcessW(
    $NvideExe,
    $commandLine,
    [IntPtr]::Zero,
    [IntPtr]::Zero,
    $false,
    0x00000004,
    [IntPtr]::Zero,
    (Split-Path -Parent $NvideExe),
    [ref]$startup,
    [ref]$processInfo)
if (!$created) {
    throw "CreateProcessW failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
}

$app = $null
$presentMon = $null
$raw = $null
$resumed = $false
$rows = 0
$acknowledgements = 0
$swapchain = $null
$headers = $null
$qpcFrequency = [Diagnostics.Stopwatch]::Frequency
$capturedFrames = New-Object Collections.Generic.List[object]
$acknowledgedRequests = @{}
$captureSeconds = if ($Kind -eq "clear") {
    $WarmupSeconds + $MeasureSeconds + 10
} else {
    [Math]::Max(15, ($WarmupEdits + $MeasureEdits) * 2)
}

try {
    $app = [Diagnostics.Process]::GetProcessById([int]$processInfo.dwProcessId)

    $presentMonStart = New-Object Diagnostics.ProcessStartInfo
    $presentMonStart.FileName = $PresentMonExe
    $presentMonStart.Arguments = @(
        "--process_id", $processInfo.dwProcessId,
        "--output_stdout",
        "--no_console_stats",
        "--qpc_time",
        "--timed", $captureSeconds,
        "--terminate_after_timed",
        "--session_name", $session
    ) -join " "
    $presentMonStart.UseShellExecute = $false
    $presentMonStart.RedirectStandardOutput = $true
    $presentMonStart.RedirectStandardError = $true
    $presentMon = New-Object Diagnostics.Process
    $presentMon.StartInfo = $presentMonStart
    if (!$presentMon.Start()) {
        throw "failed to start PresentMon"
    }

    Start-Sleep -Seconds 3
    if ($presentMon.HasExited) {
        throw "PresentMon exited before NVide resume: $($presentMon.StandardError.ReadToEnd())"
    }
    if ([Phase0Native]::ResumeThread($processInfo.hThread) -eq [uint32]::MaxValue) {
        throw "ResumeThread failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    }
    $resumed = $true
    [void][Phase0Native]::CloseHandle($processInfo.hThread)
    $processInfo.hThread = [IntPtr]::Zero

    $utf8 = New-Object Text.UTF8Encoding($false)
    $raw = New-Object IO.StreamWriter($rawPath, $false, $utf8)
    while (!$presentMon.StandardOutput.EndOfStream) {
        $line = $presentMon.StandardOutput.ReadLine()
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        if ($null -eq $headers) {
            if (!$line.StartsWith("Application,ProcessID,SwapChainAddress,")) {
                continue
            }
            $headers = $line.Split(',')
            foreach ($required in @("Application", "ProcessID", "SwapChainAddress", "PresentRuntime", "SyncInterval", "AllowsTearing", "PresentMode", "TimeInQPC", "MsUntilDisplayed")) {
                if ($headers -notcontains $required) {
                    throw "PresentMon output lacks required column: $required"
                }
            }
            $raw.WriteLine($line)
            $raw.Flush()
            continue
        }

        $raw.WriteLine($line)
        $raw.Flush()
        $row = $line | ConvertFrom-Csv -Header $headers
        if ($row.Application -ne "nvide.exe" -or [uint32]$row.ProcessID -ne $processInfo.dwProcessId) {
            throw "PresentMon row escaped the bound PID"
        }
        if ($null -eq $swapchain) {
            $swapchain = $row.SwapChainAddress
        } elseif ($row.SwapChainAddress -ne $swapchain) {
            throw "PresentMon observed multiple swapchains"
        }
        if ($row.PresentRuntime -ne "DXGI" -or $row.SyncInterval -ne "1" -or $row.AllowsTearing -ne "0") {
            throw "PresentMon row violates the FIFO/no-tearing binding"
        }

        $rows++
        $presentQpc = [decimal]([int64]$row.TimeInQPC)
        $presentNanoseconds = [uint64][Math]::Floor(
            $presentQpc * 1000000000 / [decimal]$qpcFrequency)
        $displayedNanoseconds = $null
        if ($row.MsUntilDisplayed -ne "NA") {
            $untilMilliseconds = [decimal](Parse-InvariantDouble $row.MsUntilDisplayed)
            $displayedNanoseconds = [uint64][Math]::Floor(
                ($presentQpc * 1000000000 / [decimal]$qpcFrequency) +
                ($untilMilliseconds * 1000000))
        }
        $capturedFrames.Add([pscustomobject]@{
            PresentNanoseconds = $presentNanoseconds
            DisplayedNanoseconds = $displayedNanoseconds
        })

        if ($Kind -eq "edit") {
            foreach ($requestFile in Get-ChildItem -LiteralPath $Output -Filter "displayed-request-*.csv") {
                if ($acknowledgedRequests.ContainsKey($requestFile.FullName)) {
                    continue
                }
                $request = @(Import-Csv -LiteralPath $requestFile.FullName)
                if ($request.Count -ne 1 -or [uint32]$request[0].pid -ne $processInfo.dwProcessId) {
                    throw "invalid display request: $($requestFile.Name)"
                }
                $requestedPresent = [uint64]$request[0].present_ns
                $candidates = @($capturedFrames | Where-Object {
                    [Math]::Abs(
                        [decimal]$_.PresentNanoseconds - [decimal]$requestedPresent) -le 2000000
                })
                if ($candidates.Count -gt 1) {
                    throw "ambiguous PresentMon match for $($requestFile.Name)"
                }
                if ($candidates.Count -eq 0 -or $null -eq $candidates[0].DisplayedNanoseconds) {
                    continue
                }
                $candidate = $candidates[0]
                $requestedSequence = [uint64]$request[0].frame_sequence
                $ack = "pid,frame_sequence,displayed_ns`n$($processInfo.dwProcessId),$requestedSequence,$($candidate.DisplayedNanoseconds)`n"
                $temporaryAck = "$ackPath.tmp-$($processInfo.dwProcessId)-$requestedSequence"
                [IO.File]::WriteAllText($temporaryAck, $ack, $utf8)
                if (![Phase0Native]::MoveFileExW($temporaryAck, $ackPath, 0x00000009)) {
                    throw "atomic acknowledgement replace failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
                }
                $acknowledgedRequests[$requestFile.FullName] = $true
                $acknowledgements++
            }
        }
    }

    $raw.Flush()
    $presentMon.WaitForExit()
    [IO.File]::WriteAllText($consolePath, $presentMon.StandardError.ReadToEnd(), $utf8)
    if ($presentMon.ExitCode -ne 0) {
        throw "PresentMon failed: $($presentMon.ExitCode)"
    }
    if (!$app.WaitForExit(10000)) {
        $app.Kill()
        throw "NVide did not exit after the presentation capture"
    }
    $appExitCode = 0
    if (![Phase0Native]::GetExitCodeProcess($processInfo.hProcess, [ref]$appExitCode)) {
        throw "GetExitCodeProcess failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    }
    if ($appExitCode -ne 0) {
        throw "NVide failed: $appExitCode"
    }
    if ($rows -eq 0 -or [string]::IsNullOrWhiteSpace($swapchain)) {
        throw "PresentMon captured no bound frames"
    }
    if ($Kind -eq "edit" -and $acknowledgements -lt ($WarmupEdits + $MeasureEdits)) {
        throw "PresentMon produced too few displayed acknowledgements: $acknowledgements"
    }

    $display = Get-CimInstance Win32_VideoController | Select-Object -First 1
    $presentMonName = Split-Path -Leaf $PresentMonExe
    if ($presentMonName -notmatch '^PresentMon-([0-9]+(?:\.[0-9]+)+)-x64\.exe$') {
        throw "PresentMon filename does not carry its version"
    }
    $presentMonVersion = $Matches[1]
    $captureManifest = @(
        "format=nvide-phase0-capture-v1",
        "run_id=$RunId",
        "kind=$Kind",
        "git_commit=$gitCommit",
        "pid=$($processInfo.dwProcessId)",
        "swapchain=$swapchain",
        "harness_sha256=$((Get-FileHash -Algorithm SHA256 $PSCommandPath).Hash)",
        "nvide_sha256=$((Get-FileHash -Algorithm SHA256 $NvideExe).Hash)",
        "presentmon_version=$presentMonVersion",
        "presentmon_sha256=$((Get-FileHash -Algorithm SHA256 $PresentMonExe).Hash)",
        "nvide_arguments=$($appArguments -join ' ')",
        "presentmon_arguments=$($presentMonStart.Arguments)",
        "qpc_frequency=$qpcFrequency",
        "presentmon_rows=$rows",
        "displayed_acknowledgements=$acknowledgements",
        "resolution=$($display.CurrentHorizontalResolution)x$($display.CurrentVerticalResolution)",
        "configured_refresh_hz=$($display.CurrentRefreshRate)",
        "capture_utc=$([DateTime]::UtcNow.ToString('o'))"
    ) -join "`n"
    [IO.File]::WriteAllText($captureManifestPath, "$captureManifest`n", $utf8)
} finally {
    if ($null -ne $raw) {
        $raw.Dispose()
    }
    if (!$resumed -and $processInfo.hThread -ne [IntPtr]::Zero) {
        [void][Phase0Native]::ResumeThread($processInfo.hThread)
    }
    if ($processInfo.hThread -ne [IntPtr]::Zero) {
        [void][Phase0Native]::CloseHandle($processInfo.hThread)
    }
    if ($processInfo.hProcess -ne [IntPtr]::Zero) {
        [void][Phase0Native]::CloseHandle($processInfo.hProcess)
    }
    if ($null -ne $app -and !$app.HasExited) {
        $app.Kill()
    }
}
