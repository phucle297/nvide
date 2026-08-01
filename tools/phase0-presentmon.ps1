[CmdletBinding(DefaultParameterSetName = "Capture")]
param(
    [Parameter(Mandatory = $true, ParameterSetName = "SelfTest")]
    [switch]$SelfTest,
    [Parameter(Mandatory = $true, ParameterSetName = "Capture")]
    [ValidateSet("clear", "edit")]
    [string]$Kind,
    [Parameter(Mandatory = $true, ParameterSetName = "Capture")]
    [string]$RunId,
    [Parameter(Mandatory = $true, ParameterSetName = "Capture")]
    [string]$Output,
    [Parameter(Mandatory = $true, ParameterSetName = "Capture")]
    [string]$NvideExe,
    [Parameter(Mandatory = $true, ParameterSetName = "Capture")]
    [string]$PresentMonExe,
    [Parameter(ParameterSetName = "Capture")]
    [int]$WarmupSeconds = 10,
    [Parameter(ParameterSetName = "Capture")]
    [int]$MeasureSeconds = 30,
    [Parameter(ParameterSetName = "Capture")]
    [int]$WarmupEdits = 10,
    [Parameter(ParameterSetName = "Capture")]
    [int]$MeasureEdits = 30,
    [Parameter(ParameterSetName = "Capture")]
    [switch]$UnboundDiagnostic
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$requestHeader = "pid,trace_id,frame_sequence,present_ns"
$presentMonHeader = "Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,Dropped,TimeInSeconds,msInPresentAPI,msBetweenPresents,AllowsTearing,PresentMode,msUntilRenderComplete,msUntilDisplayed,msBetweenDisplayChange,msFlipDelay,msUntilRenderStart,msGPUActive,msSinceInput,QPCTime"

function Parse-UInt32([string]$Value, [string]$Name) {
    $parsed = [uint32]0
    if (![uint32]::TryParse($Value, [Globalization.NumberStyles]::None, [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed)) {
        throw "invalid decimal $Name"
    }
    return $parsed
}

function Parse-UInt64([string]$Value, [string]$Name) {
    $parsed = [uint64]0
    if (![uint64]::TryParse($Value, [Globalization.NumberStyles]::None, [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed)) {
        throw "invalid decimal $Name"
    }
    return $parsed
}

function Parse-InvariantDouble([string]$Value, [string]$Name) {
    $parsed = [double]0
    if (![double]::TryParse($Value, [Globalization.NumberStyles]::Float, [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed) -or [double]::IsNaN($parsed) -or [double]::IsInfinity($parsed)) {
        throw "invalid $Name"
    }
    return $parsed
}

function Read-DisplayRequest([IO.FileInfo]$File, [uint32]$ExpectedPid) {
    if ($File.Name -notmatch '^displayed-request-([1-9][0-9]*)\.csv$') {
        throw "invalid display request filename: $($File.Name)"
    }
    $filenameSequence = Parse-UInt64 $Matches[1] "request filename sequence"
    $lines = [IO.File]::ReadAllLines($File.FullName)
    if ($lines.Count -ne 2 -or $lines[0] -cne $script:requestHeader) {
        throw "invalid display request schema: $($File.Name)"
    }
    $fields = $lines[1].Split(',')
    if ($fields.Count -ne 4) {
        throw "invalid display request row: $($File.Name)"
    }
    $requestPid = Parse-UInt32 $fields[0] "request PID"
    $traceId = Parse-UInt64 $fields[1] "request trace ID"
    $sequence = Parse-UInt64 $fields[2] "request frame sequence"
    $presentNanoseconds = Parse-UInt64 $fields[3] "request present timestamp"
    if ($requestPid -ne $ExpectedPid -or $traceId -eq 0 -or $sequence -ne $filenameSequence -or $presentNanoseconds -eq 0) {
        throw "display request identity mismatch: $($File.Name)"
    }
    return [pscustomobject]@{
        File = $File.FullName
        Hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $File.FullName).Hash
        TraceId = $traceId
        Sequence = $sequence
        PresentNanoseconds = $presentNanoseconds
    }
}

function Register-DisplayRequest([object]$Request, [hashtable]$BySequence, [hashtable]$TraceIds) {
    if ($BySequence.ContainsKey($Request.Sequence)) {
        $existing = $BySequence[$Request.Sequence]
        if ($existing.File -ne $Request.File -or $existing.Hash -ne $Request.Hash) {
            throw "duplicate or changed display request sequence: $($Request.Sequence)"
        }
        return
    }
    if ($TraceIds.ContainsKey($Request.TraceId)) {
        throw "duplicate display request trace ID: $($Request.TraceId)"
    }
    $BySequence[$Request.Sequence] = $Request
    $TraceIds[$Request.TraceId] = $true
}

function Get-PresentMatches([Collections.IEnumerable]$Frames, [uint64]$PresentNanoseconds) {
    return @($Frames | Where-Object {
        [Math]::Abs([decimal]$_.PresentNanoseconds - [decimal]$PresentNanoseconds) -le 2000000
    })
}

function Assert-PresentMonHeader([string]$Header) {
    if ($Header -cne $script:presentMonHeader) {
        throw "PresentMon output does not match the exact 2.5.1 v1 schema"
    }
}

function Assert-PresentRow([object]$Row, [uint32]$ExpectedPid, [string]$ExpectedSwapchain) {
    if ($Row.Application -cne "nvide.exe" -or (Parse-UInt32 $Row.ProcessID "PresentMon PID") -ne $ExpectedPid) {
        throw "PresentMon row escaped the bound PID"
    }
    if ($Row.SwapChainAddress -notmatch '^0x[0-9A-Fa-f]+$') {
        throw "PresentMon row has an invalid swapchain"
    }
    if ($ExpectedSwapchain -and $Row.SwapChainAddress -cne $ExpectedSwapchain) {
        throw "PresentMon observed multiple swapchains"
    }
    if ($Row.Runtime -cne "DXGI" -or $Row.SyncInterval -cne "1" -or $Row.AllowsTearing -cne "0" -or $Row.Dropped -notin @("0", "1")) {
        throw "PresentMon row violates the DXGI FIFO/no-tearing binding"
    }
}

function Convert-PresentFrame([object]$Row, [uint64]$QpcFrequency) {
    $qpc = Parse-UInt64 $Row.QPCTime "PresentMon QPC time"
    $presentNanoseconds = [uint64][Math]::Floor([decimal]$qpc * 1000000000 / [decimal]$QpcFrequency)
    $displayedNanoseconds = $null
    if ($Row.Dropped -eq "0") {
        $untilMilliseconds = Parse-InvariantDouble $Row.msUntilDisplayed "PresentMon display offset"
        if ($untilMilliseconds -lt 0) { throw "negative PresentMon display offset" }
        $displayedNanoseconds = [uint64][Math]::Floor([decimal]$presentNanoseconds + ([decimal]$untilMilliseconds * 1000000))
    }
    return [pscustomobject]@{ PresentNanoseconds = $presentNanoseconds; DisplayedNanoseconds = $displayedNanoseconds }
}

function Assert-CompleteJoin([hashtable]$Requests, [Collections.IEnumerable]$Frames, [hashtable]$Acknowledged, [int]$ExpectedCount) {
    if ($Requests.Count -ne $ExpectedCount -or $Acknowledged.Count -ne $ExpectedCount) {
        throw "expected exactly $ExpectedCount unique requests and acknowledgements"
    }
    foreach ($request in $Requests.Values) {
        $matches = @(Get-PresentMatches $Frames $request.PresentNanoseconds)
        if ($matches.Count -ne 1 -or $null -eq $matches[0].DisplayedNanoseconds) {
            throw "display request $($request.Sequence) does not have one unique displayed match"
        }
    }
}

function Assert-PresentationExit([int]$ExitCode, [bool]$StoppedAfterApplicationExit) {
    if ($ExitCode -ne 0 -and !($ExitCode -eq -1 -and $StoppedAfterApplicationExit)) {
        throw "PresentMon failed: $ExitCode"
    }
}

function Should-StopAfterApplicationExit([int]$ExitCode, [int64]$ExitObservedMs, [int64]$LastOutputMs, [int64]$NowMs) {
    return $ExitCode -ne 0 -or $NowMs - $LastOutputMs -ge 1000 -or $NowMs - $ExitObservedMs -ge 5000
}

function Should-RetryAtomicReplace([int]$ErrorCode, [int64]$ElapsedMs) {
    return ($ErrorCode -eq 5 -or $ErrorCode -eq 32) -and $ElapsedMs -lt 250
}

function New-EvidenceDirectory([string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        throw "evidence output already exists: $Path"
    }
    New-Item -ItemType Directory -Path $Path -ErrorAction Stop | Out-Null
}

function Stop-ProcessSafely([Diagnostics.Process]$Process) {
    if ($null -ne $Process -and !$Process.HasExited) {
        $Process.Kill()
        if (!$Process.WaitForExit(5000)) {
            throw "process did not stop after kill"
        }
    }
}

function Stop-PresentationCapture([Diagnostics.Process]$Process, [string]$Executable, [string]$Session) {
    if ($null -eq $Process -or $Process.HasExited) {
        return
    }
    try {
        $terminator = Start-Process -FilePath $Executable -ArgumentList @("--session_name", $Session, "--terminate_existing_session") -PassThru -NoNewWindow
        if (!$terminator.WaitForExit(5000)) {
            Stop-ProcessSafely $terminator
        }
    } catch {
        # The exact capture process is still terminated below.
    }
    if (!$Process.WaitForExit(5000)) {
        Stop-ProcessSafely $Process
    }
}

function Invoke-SelfTest {
    $root = Join-Path ([IO.Path]::GetTempPath()) ("nvide-phase0-harness-" + [guid]::NewGuid())
    New-EvidenceDirectory $root
    try {
        $valid = Join-Path $root "displayed-request-1.csv"
        [IO.File]::WriteAllText($valid, "$script:requestHeader`n7,1,1,100`n")
        $requests = @{}
        $traces = @{}
        Register-DisplayRequest (Read-DisplayRequest (Get-Item $valid) 7) $requests $traces
        if ($requests.Count -ne 1) { throw "valid request fixture failed" }

        [IO.File]::WriteAllText($valid, "bad-header`n7,1,1,100`n")
        try { Read-DisplayRequest (Get-Item $valid) 7; throw "invalid header fixture passed" } catch { if ($_.Exception.Message -eq "invalid header fixture passed") { throw } }
        [IO.File]::WriteAllText($valid, "$script:requestHeader`n7,x,1,100`n")
        try { Read-DisplayRequest (Get-Item $valid) 7; throw "invalid ID fixture passed" } catch { if ($_.Exception.Message -eq "invalid ID fixture passed") { throw } }
        [IO.File]::WriteAllText($valid, "$script:requestHeader`n7,1,2,100`n")
        try { Read-DisplayRequest (Get-Item $valid) 7; throw "filename mismatch fixture passed" } catch { if ($_.Exception.Message -eq "filename mismatch fixture passed") { throw } }

        [IO.File]::WriteAllText($valid, "$script:requestHeader`n7,1,1,100`n")
        $duplicateTrace = Join-Path $root "displayed-request-2.csv"
        [IO.File]::WriteAllText($duplicateTrace, "$script:requestHeader`n7,1,2,200`n")
        try { Register-DisplayRequest (Read-DisplayRequest (Get-Item $duplicateTrace) 7) $requests $traces; throw "duplicate trace fixture passed" } catch { if ($_.Exception.Message -eq "duplicate trace fixture passed") { throw } }

        Assert-PresentMonHeader $script:presentMonHeader
        try { Assert-PresentMonHeader "Application,bad"; throw "PresentMon header fixture passed" } catch { if ($_.Exception.Message -eq "PresentMon header fixture passed") { throw } }
        $row = "$script:presentMonHeader`nnvide.exe,7,0x1,DXGI,1,0,0,0,0,0,0,Composed: Flip,0,1.0,0,0,0,0,0,10" | ConvertFrom-Csv
        Assert-PresentRow $row 7 ""
        if ($null -eq (Convert-PresentFrame $row 10).DisplayedNanoseconds) { throw "displayed fixture failed" }
        $dropped = "$script:presentMonHeader`nnvide.exe,7,0x1,DXGI,1,0,1,0,0,0,0,Composed: Flip,0,0,0,0,0,0,0,11" | ConvertFrom-Csv
        Assert-PresentRow $dropped 7 "0x1"
        if ($null -ne (Convert-PresentFrame $dropped 10).DisplayedNanoseconds) { throw "dropped fixture failed" }
        try { Assert-PresentRow $row 7 "0x2"; throw "swapchain fixture passed" } catch { if ($_.Exception.Message -eq "swapchain fixture passed") { throw } }
        $one = @([pscustomobject]@{ PresentNanoseconds = [uint64]100; DisplayedNanoseconds = [uint64]110 })
        $two = @($one[0], [pscustomobject]@{ PresentNanoseconds = [uint64]101; DisplayedNanoseconds = [uint64]111 })
        if (@(Get-PresentMatches $one 100).Count -ne 1 -or @(Get-PresentMatches $two 100).Count -ne 2) { throw "join fixture failed" }
        Assert-CompleteJoin $requests $one @{ 1 = $true } 1
        try { Assert-CompleteJoin $requests $two @{ 1 = $true } 1; throw "ambiguous join fixture passed" } catch { if ($_.Exception.Message -eq "ambiguous join fixture passed") { throw } }
        try { Assert-CompleteJoin $requests $one @{} 1; throw "exact count fixture passed" } catch { if ($_.Exception.Message -eq "exact count fixture passed") { throw } }
        Assert-PresentationExit -1 $true
        try { Assert-PresentationExit -1 $false; throw "authority exit fixture passed" } catch { if ($_.Exception.Message -eq "authority exit fixture passed") { throw } }
        try { Assert-PresentationExit 5 $true; throw "unexpected authority exit fixture passed" } catch { if ($_.Exception.Message -eq "unexpected authority exit fixture passed") { throw } }
        if (Should-StopAfterApplicationExit 0 0 500 1499) { throw "quiet-drain fixture stopped early" }
        if (!(Should-StopAfterApplicationExit 0 0 500 1500)) { throw "quiet-drain fixture did not stop" }
        if (!(Should-StopAfterApplicationExit 5 0 0 0)) { throw "failed-application fixture did not stop" }
        if (!(Should-StopAfterApplicationExit 0 0 4999 5000)) { throw "drain-cap fixture did not stop" }
        if (!(Should-RetryAtomicReplace 5 249)) { throw "access-denied retry fixture did not retry" }
        if (!(Should-RetryAtomicReplace 32 0)) { throw "sharing-violation retry fixture did not retry" }
        if (Should-RetryAtomicReplace 5 250) { throw "atomic-replace retry fixture exceeded its cap" }
        if (Should-RetryAtomicReplace 2 0) { throw "unexpected atomic-replace error was retried" }

        $child = Start-Process -FilePath powershell.exe -ArgumentList @("-NoProfile", "-Command", "Start-Sleep -Seconds 30") -PassThru
        Stop-ProcessSafely $child
        if (!$child.HasExited) { throw "cleanup fixture failed" }
        try { New-EvidenceDirectory $root; throw "output reuse fixture passed" } catch { if ($_.Exception.Message -eq "output reuse fixture passed") { throw } }
        Write-Output "phase0-presentmon self-test passed"
    } finally {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($SelfTest) {
    Invoke-SelfTest
    exit 0
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class Phase0Native {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct STARTUPINFO {
        public uint cb; public string lpReserved; public string lpDesktop; public string lpTitle;
        public uint dwX; public uint dwY; public uint dwXSize; public uint dwYSize;
        public uint dwXCountChars; public uint dwYCountChars; public uint dwFillAttribute;
        public uint dwFlags; public ushort wShowWindow; public ushort cbReserved2;
        public IntPtr lpReserved2; public IntPtr hStdInput; public IntPtr hStdOutput; public IntPtr hStdError;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct PROCESS_INFORMATION {
        public IntPtr hProcess; public IntPtr hThread; public uint dwProcessId; public uint dwThreadId;
    }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool CreateProcessW(string applicationName, StringBuilder commandLine,
        IntPtr processAttributes, IntPtr threadAttributes, bool inheritHandles, uint creationFlags,
        IntPtr environment, string currentDirectory, ref STARTUPINFO startupInfo,
        out PROCESS_INFORMATION processInformation);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool MoveFileExW(string existingName, string newName, uint flags);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetWindowPos(IntPtr window, IntPtr insertAfter,
        int x, int y, int width, int height, uint flags);
}
"@

function Set-BenchmarkWindowTopmost([Diagnostics.Process]$Process) {
    $wait = [Diagnostics.Stopwatch]::StartNew()
    while ($wait.ElapsedMilliseconds -lt 5000) {
        if ($Process.HasExited) { throw "NVide exited before its benchmark window appeared" }
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
            if (![Phase0Native]::SetWindowPos($Process.MainWindowHandle, [IntPtr](-1), 0, 0, 0, 0, 0x00000053)) {
                throw "SetWindowPos failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
            }
            return
        }
        Start-Sleep -Milliseconds 50
    }
    throw "NVide benchmark window did not appear within five seconds"
}

function Quote-Argument([string]$Value) {
    return '"' + $Value.Replace('"', '\"') + '"'
}

$NvideExe = [IO.Path]::GetFullPath($NvideExe)
$PresentMonExe = [IO.Path]::GetFullPath($PresentMonExe)
$Output = [IO.Path]::GetFullPath($Output)
if (!(Test-Path -LiteralPath $NvideExe -PathType Leaf)) { throw "NVide executable not found: $NvideExe" }
if (!(Test-Path -LiteralPath $PresentMonExe -PathType Leaf)) { throw "PresentMon executable not found: $PresentMonExe" }
$repoRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $NvideExe))
$git = (Get-Command git.exe -ErrorAction Stop).Source
$gitCommit = (& $git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $gitCommit -notmatch '^[0-9a-f]{40}$') { throw "cannot resolve the benchmark commit" }
$trackedChanges = & $git -C $repoRoot status --porcelain --untracked-files=no
if ($LASTEXITCODE -ne 0 -or $trackedChanges) { throw "the benchmark checkout has tracked changes" }
$presentMonName = Split-Path -Leaf $PresentMonExe
if ($presentMonName -notmatch '^PresentMon-([0-9]+(?:\.[0-9]+)+)-x64\.exe$') { throw "PresentMon filename does not carry its version" }
$presentMonVersion = $Matches[1]
if ($presentMonVersion -ne "2.5.1") { throw "the approved harness requires PresentMon 2.5.1" }
New-EvidenceDirectory $Output

$rawPath = Join-Path $Output "presentmon.csv"
$consolePath = Join-Path $Output "presentmon-stderr.txt"
$captureManifestPath = Join-Path $Output "capture-manifest.txt"
$ackPath = Join-Path $Output "displayed-ack.csv"
$session = "nvide-$($RunId -replace '[^A-Za-z0-9]', '-')"
$appArguments = if ($Kind -eq "clear") {
    @("--phase0-benchmark", "clear", "--run-id", $RunId, "--output", $Output, "--warmup-seconds", $WarmupSeconds, "--measure-seconds", $MeasureSeconds)
} else {
    @("--phase0-benchmark", "edit", "--run-id", $RunId, "--output", $Output, "--warmup-edits", $WarmupEdits, "--measure-edits", $MeasureEdits)
}
if ($UnboundDiagnostic) { $appArguments += "--unbound-diagnostic" }
$captureSeconds = if ($Kind -eq "clear") { $WarmupSeconds + $MeasureSeconds + 10 } else { [Math]::Max(15, ($WarmupEdits + $MeasureEdits) * 5 + 10) }
$presentMonArguments = @("--process_id", "PID", "--output_stdout", "--no_console_stats", "--qpc_time", "--v1_metrics", "--timed", $captureSeconds, "--terminate_after_timed", "--session_name", $session)

$commandLine = New-Object Text.StringBuilder
[void]$commandLine.Append((Quote-Argument $NvideExe))
foreach ($argument in $appArguments) { [void]$commandLine.Append(" "); [void]$commandLine.Append((Quote-Argument ([string]$argument))) }

$startup = New-Object Phase0Native+STARTUPINFO
$startup.cb = [Runtime.InteropServices.Marshal]::SizeOf($startup)
$processInfo = New-Object Phase0Native+PROCESS_INFORMATION
$app = $null
$presentMon = $null
$stderrTask = $null
$raw = $null
$resumed = $false
$stoppedAfterApplicationExit = $false
$postExitDrainMilliseconds = 0
$completed = $false
$failureMessage = "capture did not complete"
$rows = 0
$swapchain = $null
$qpcFrequency = [Diagnostics.Stopwatch]::Frequency
$capturedFrames = New-Object Collections.Generic.List[object]
$requests = @{}
$requestTraceIds = @{}
$acknowledged = @{}
$utf8 = New-Object Text.UTF8Encoding($false)

try {
    $created = [Phase0Native]::CreateProcessW($NvideExe, $commandLine, [IntPtr]::Zero, [IntPtr]::Zero,
        $false, 0x00000004, [IntPtr]::Zero, (Split-Path -Parent $NvideExe), [ref]$startup, [ref]$processInfo)
    if (!$created) { throw "CreateProcessW failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())" }
    $app = [Diagnostics.Process]::GetProcessById([int]$processInfo.dwProcessId)
    $presentMonArguments[1] = [string]$processInfo.dwProcessId
    $presentMonStart = New-Object Diagnostics.ProcessStartInfo
    $presentMonStart.FileName = $PresentMonExe
    $presentMonStart.Arguments = $presentMonArguments -join " "
    $presentMonStart.UseShellExecute = $false
    $presentMonStart.RedirectStandardOutput = $true
    $presentMonStart.RedirectStandardError = $true
    $presentMon = New-Object Diagnostics.Process
    $presentMon.StartInfo = $presentMonStart
    if (!$presentMon.Start()) { throw "failed to start PresentMon" }
    $stderrTask = $presentMon.StandardError.ReadToEndAsync()

    Start-Sleep -Seconds 3
    if ($presentMon.HasExited) { throw "PresentMon exited before NVide resume" }
    if ([Phase0Native]::ResumeThread($processInfo.hThread) -eq [uint32]::MaxValue) { throw "ResumeThread failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())" }
    $resumed = $true
    [void][Phase0Native]::CloseHandle($processInfo.hThread)
    $processInfo.hThread = [IntPtr]::Zero
    Set-BenchmarkWindowTopmost $app

    $raw = New-Object IO.StreamWriter($rawPath, $false, $utf8)
    $headerSeen = $false
    $streamClock = [Diagnostics.Stopwatch]::StartNew()
    $lastOutputMilliseconds = [int64]0
    $applicationExitObservedMilliseconds = $null
    while ($true) {
        $lineTask = $presentMon.StandardOutput.ReadLineAsync()
        while (!$lineTask.Wait(100)) {
            if ($app.HasExited -and !$presentMon.HasExited) {
                $nowMilliseconds = $streamClock.ElapsedMilliseconds
                if ($null -eq $applicationExitObservedMilliseconds) {
                    $applicationExitObservedMilliseconds = $nowMilliseconds
                }
                if (Should-StopAfterApplicationExit $app.ExitCode $applicationExitObservedMilliseconds $lastOutputMilliseconds $nowMilliseconds) {
                    $postExitDrainMilliseconds = $nowMilliseconds - $applicationExitObservedMilliseconds
                    $stoppedAfterApplicationExit = $true
                    Stop-PresentationCapture $presentMon $PresentMonExe $session
                }
            }
        }
        $line = $lineTask.GetAwaiter().GetResult()
        if ($null -eq $line) { break }
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $lastOutputMilliseconds = $streamClock.ElapsedMilliseconds
        if (!$headerSeen) {
            if (!$line.StartsWith("Application,")) { continue }
            Assert-PresentMonHeader $line
            $headerSeen = $true
            $headers = $line.Split(',')
            $raw.WriteLine($line); $raw.Flush()
            continue
        }
        $raw.WriteLine($line); $raw.Flush()
        $row = $line | ConvertFrom-Csv -Header $headers
        Assert-PresentRow $row $processInfo.dwProcessId $swapchain
        if ($null -eq $swapchain) { $swapchain = $row.SwapChainAddress }
        $capturedFrames.Add((Convert-PresentFrame $row $qpcFrequency))
        $rows++

        if ($Kind -eq "edit") {
            foreach ($requestFile in Get-ChildItem -LiteralPath $Output -Filter "displayed-request-*.csv") {
                $request = Read-DisplayRequest $requestFile $processInfo.dwProcessId
                Register-DisplayRequest $request $requests $requestTraceIds
                if ($acknowledged.ContainsKey($request.Sequence)) { continue }
                $matches = @(Get-PresentMatches $capturedFrames $request.PresentNanoseconds)
                if ($matches.Count -gt 1) { throw "ambiguous PresentMon match for $($request.Sequence)" }
                if ($matches.Count -eq 0 -or $null -eq $matches[0].DisplayedNanoseconds) { continue }
                $ack = "pid,frame_sequence,displayed_ns`n$($processInfo.dwProcessId),$($request.Sequence),$($matches[0].DisplayedNanoseconds)`n"
                $temporaryAck = "$ackPath.tmp-$($processInfo.dwProcessId)-$($request.Sequence)"
                [IO.File]::WriteAllText($temporaryAck, $ack, $utf8)
                $replaceWait = [Diagnostics.Stopwatch]::StartNew()
                while (![Phase0Native]::MoveFileExW($temporaryAck, $ackPath, 0x00000009)) {
                    $replaceError = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
                    if (!(Should-RetryAtomicReplace $replaceError $replaceWait.ElapsedMilliseconds)) {
                        throw "atomic acknowledgement replace failed: $replaceError"
                    }
                    Start-Sleep -Milliseconds 5
                }
                $acknowledged[$request.Sequence] = $true
            }
        }
    }

    if (!$headerSeen -or $rows -eq 0 -or [string]::IsNullOrWhiteSpace($swapchain)) { throw "PresentMon captured no bound frames with the exact v1 schema" }
    if (!$app.WaitForExit(10000)) { throw "NVide did not exit after the presentation capture" }
    $appExitCode = [uint32]0
    if (![Phase0Native]::GetExitCodeProcess($processInfo.hProcess, [ref]$appExitCode)) { throw "GetExitCodeProcess failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())" }
    if ($appExitCode -ne 0) { throw "NVide failed: $appExitCode" }
    Assert-PresentationExit $presentMon.ExitCode $stoppedAfterApplicationExit
    if ($Kind -eq "edit") { Assert-CompleteJoin $requests $capturedFrames $acknowledged ($WarmupEdits + $MeasureEdits) }
    $completed = $true
    $failureMessage = ""
} catch {
    $failureMessage = $_.Exception.Message.Replace("`r", " ").Replace("`n", " ")
    throw
} finally {
    if (!$resumed -and $processInfo.hThread -ne [IntPtr]::Zero) { [void][Phase0Native]::ResumeThread($processInfo.hThread) }
    if ($processInfo.hThread -ne [IntPtr]::Zero) { [void][Phase0Native]::CloseHandle($processInfo.hThread) }
    if ($null -ne $app -and !$app.HasExited) { Stop-ProcessSafely $app }
    Stop-PresentationCapture $presentMon $PresentMonExe $session
    if ($null -ne $raw) { $raw.Flush(); $raw.Dispose() }
    $stderr = if ($null -ne $stderrTask) { $stderrTask.GetAwaiter().GetResult() } else { "" }
    [IO.File]::WriteAllText($consolePath, $stderr, $utf8)
    if ($processInfo.hProcess -ne [IntPtr]::Zero) { [void][Phase0Native]::CloseHandle($processInfo.hProcess) }
    $display = Get-CimInstance Win32_VideoController | Select-Object -First 1
    $manifest = @(
        "format=nvide-phase0-capture-v2", "status=$(if ($completed) { 'PASS' } else { 'FAILED' })",
        "failure=$failureMessage", "run_id=$RunId", "kind=$Kind", "git_commit=$gitCommit",
        "pid=$($processInfo.dwProcessId)", "swapchain=$swapchain",
        "harness_sha256=$((Get-FileHash -Algorithm SHA256 $PSCommandPath).Hash)",
        "nvide_sha256=$((Get-FileHash -Algorithm SHA256 $NvideExe).Hash)",
        "presentmon_version=$presentMonVersion", "presentmon_sha256=$((Get-FileHash -Algorithm SHA256 $PresentMonExe).Hash)",
        "nvide_arguments=$($appArguments -join ' ')", "presentmon_arguments=$($presentMonArguments -join ' ')",
        "qpc_frequency=$qpcFrequency", "presentmon_rows=$rows", "displayed_acknowledgements=$($acknowledged.Count)",
        "presentmon_exit_code=$(if ($null -ne $presentMon -and $presentMon.HasExited) { $presentMon.ExitCode } else { '' })",
        "presentmon_stopped_after_application_exit=$stoppedAfterApplicationExit",
        "presentmon_post_exit_drain_ms=$postExitDrainMilliseconds",
        "nvide_window_topmost=True",
        "resolution=$($display.CurrentHorizontalResolution)x$($display.CurrentVerticalResolution)",
        "configured_refresh_hz=$($display.CurrentRefreshRate)", "capture_utc=$([DateTime]::UtcNow.ToString('o'))"
    ) -join "`n"
    [IO.File]::WriteAllText($captureManifestPath, "$manifest`n", $utf8)
}
