# ja-netfilter-desktop - Windows PowerShell core installer logic.
# Mirrors the techniques of ckey_script.ps1 (CodeKey Run):
#   1. Agent referenced IN-PLACE from the app's bundled resources (zero copy).
#      This script receives the final agent jar path (resolved by Rust).
#   2. Revert vmoptions: strip ALL existing -javaagent lines before appending
#      (ckey regex: ^-javaagent:.*\.jar.*), fixing broken/half-installed states.
#      Legacy -Dja.netfilter.name= lines are stripped too (no longer written).
#   3. REMOVE (not set) <PRODUCT>_VM_OPTIONS env vars from User AND Machine
#      scope so stale pointers cannot shadow the IDE's own vmoptions, then
#      broadcast WM_SETTINGCHANGE.
#   4. Locate the real IDE via %LOCALAPPDATA%\JetBrains\<Product>\.home ->
#      <install>\bin\*.vmoptions (recursive), plus the Roaming config dir
#      <APPDATA%\JetBrains\<Product>\<prd>64.exe.vmoptions.
#   5. Resolve an 8.3 short path when the agent path contains whitespace or
#      non-ASCII characters; if that is still unsafe, report "unsafe-agent-path"
#      so the Rust side can fall back to copying the agent to a space-free dir.
#
# Output contract: a single JSON object on stdout, nothing else.
# Keep this file ASCII-only: PS 5.1 parses BOM-less files in the ANSI codepage.

param(
    [Parameter(Mandatory = $true)][string]$Mode,   # install | uninstall | cleanup-env
    [string]$Products = "",                        # comma separated product ids
    [string]$AgentJar = ""                         # bundled agent jar path (in-place)
)

$ErrorActionPreference = 'Continue'
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$script:Logs = New-Object System.Collections.ArrayList
function Add-Log([string]$level, [string]$message) {
    $null = $script:Logs.Add(@{ level = $level; message = $message })
}

# ---------------------------------------------------------------------------
# Regexes (same semantics as ckey_script.ps1)
# ---------------------------------------------------------------------------
$script:AgentLineRegex = New-Object System.Text.RegularExpressions.Regex `
    '^\s*-javaagent:.*\.jar.*', `
    ([System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor [System.Text.RegularExpressions.RegexOptions]::Compiled)
$script:NameLineRegex = New-Object System.Text.RegularExpressions.Regex `
    '^\s*-Dja\.netfilter\.name=.*', `
    ([System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor [System.Text.RegularExpressions.RegexOptions]::Compiled)
# whitespace or any non-ASCII char -> path is not safe for vmoptions
$script:UnsafePathRegex = New-Object System.Text.RegularExpressions.Regex '[\s\u0080-\uFFFF]', Compiled
# ---------------------------------------------------------------------------
# 8.3 short path (defense in depth; the agent is normally referenced in-place
# from the app's own resources)
# ---------------------------------------------------------------------------
function Get-ShortPath([string]$path) {
    if (-not $script:UnsafePathRegex.IsMatch($path)) { return $path }
    try {
        $fso = New-Object -ComObject Scripting.FileSystemObject
        if ($fso.FileExists($path)) {
            $short = $fso.GetFile($path).ShortPath
        } elseif ($fso.FolderExists($path)) {
            $short = $fso.GetFolder($path).ShortPath
        } else {
            return $path
        }
        if ($short -and (-not $script:UnsafePathRegex.IsMatch($short))) {
            Add-Log "info" "short path: $short"
            return $short
        }
    } catch {
        Add-Log "warn" "short path resolution failed: $($_.Exception.Message)"
    }
    return $path
}

# ---------------------------------------------------------------------------
# Product directory discovery (ckey_script.ps1 Process_Vm_Options / Is_Product)
# ---------------------------------------------------------------------------
# Returns a list of hashtables:
#   @{ name; configDir; homeDir; binVmOptions = @(paths); roamingVmOptions = @(paths) }
function Find-ProductLocations([string]$prd) {
    $out = New-Object System.Collections.ArrayList
    $nameLower = $prd.ToLower()

    # local roots: JetBrains + vendor specific (Android Studio / DevEco)
    $rootSpecs = @(
        @{ root = Join-Path $env:LOCALAPPDATA "JetBrains";     roaming = Join-Path $env:APPDATA "JetBrains" }
    )
    if ($prd -eq 'studio') {
        $rootSpecs += @{ root = Join-Path $env:LOCALAPPDATA "Google"; roaming = Join-Path $env:APPDATA "Google" }
        $nameLower = 'androidstudio'
    } elseif ($prd -eq 'devecostudio') {
        $rootSpecs += @{ root = Join-Path $env:LOCALAPPDATA "Huawei"; roaming = Join-Path $env:APPDATA "Huawei" }
    }
    # "studio" must not swallow DevEcoStudio* dirs
    $excludeLower = ''
    if ($prd -eq 'studio') { $excludeLower = 'devecostudio' }

    foreach ($spec in $rootSpecs) {
        if (-not (Test-Path -LiteralPath $spec.root)) {
            Add-Log "debug" "scan root not present: $($spec.root)"
            continue
        }
        Add-Log "debug" "scan root: $($spec.root)"
        $dirs = Get-ChildItem -LiteralPath $spec.root -Directory -ErrorAction SilentlyContinue | Where-Object {
            $n = $_.Name.ToLower()
            if ($n.Contains($nameLower)) {
                if ($excludeLower -and $n.Contains($excludeLower)) { return $false }
                return $true
            }
            return $false
        }

        foreach ($dir in $dirs) {
            Add-Log "debug" "product config dir: $($dir.FullName)"
            $entry = @{
                name             = $dir.Name
                configDir        = $dir.FullName
                homeDir          = ''
                binVmOptions     = @()
                roamingVmOptions = @()
            }

            # .home -> real install dir -> bin\*.vmoptions (recursive)
            $homeFile = Join-Path $dir.FullName ".home"
            if (Test-Path -LiteralPath $homeFile) {
                try {
                    $installDir = (Get-Content -LiteralPath $homeFile -TotalCount 1 -ErrorAction Stop).Trim()
                    if ($installDir -and (Test-Path -LiteralPath $installDir)) {
                        $entry.homeDir = $installDir
                        Add-Log "debug" "install dir via .home: $installDir"
                        $binDir = Join-Path $installDir "bin"
                        if (Test-Path -LiteralPath $binDir) {
                            $vms = Get-ChildItem -LiteralPath $binDir -Filter *.vmoptions -Recurse -File -ErrorAction SilentlyContinue
                            $entry.binVmOptions = @($vms | ForEach-Object { $_.FullName })
                            Add-Log "debug" "bin vmoptions found: $($entry.binVmOptions.Count) file(s)"
                        }
                    } else {
                        Add-Log "warn" ".home target not found: $installDir (product dir: $($dir.Name))"
                    }
                } catch {
                    Add-Log "warn" "read .home failed for $($dir.Name): $($_.Exception.Message)"
                }
            }

            # Roaming config dir: <prd>64.exe.vmoptions / <prd>.exe.vmoptions / <prd>.vmoptions
            $roamingDir = Join-Path $spec.roaming $dir.Name
            if (Test-Path -LiteralPath $roamingDir) {
                $candidates = @()
                if ($nameLower -eq 'androidstudio') {
                    $candidates = @('studio64.exe.vmoptions', 'studio.exe.vmoptions', 'studio.vmoptions')
                } else {
                    $candidates = @("$prd`64.exe.vmoptions", "$prd.exe.vmoptions", "$prd.vmoptions")
                }
                $found = @()
                foreach ($c in $candidates) {
                    $p = Join-Path $roamingDir $c
                    if (Test-Path -LiteralPath $p) { $found += $p }
                }
                $entry.roamingVmOptions = $found
                Add-Log "debug" "roaming vmoptions found: $($found.Count) file(s) under $roamingDir"
            }

            $null = $out.Add($entry)
        }
    }
    return ,$out
}

# ---------------------------------------------------------------------------
# vmoptions editing (Revert_Vm_Options / Append_Vm_Options from ckey_script.ps1)
# ---------------------------------------------------------------------------
function Repair-VmOptionsFile([string]$path, [string]$agentLine) {
    try {
        # ReadAllLines/WriteAllLines default to UTF-8 (no BOM) - safe for vmoptions
        $lines = [System.IO.File]::ReadAllLines($path)
        $kept = @($lines | Where-Object {
            (-not $script:AgentLineRegex.IsMatch($_)) -and
            (-not $script:NameLineRegex.IsMatch($_))
        })
        $removed = $lines.Count - $kept.Count
        $all = New-Object System.Collections.Generic.List[string]
        foreach ($l in $kept) { $null = $all.Add($l) }
        $null = $all.Add($agentLine)
        [System.IO.File]::WriteAllLines($path, $all)
        if ($removed -gt 0) {
            Add-Log "info" "cleaned $removed stale agent line(s) from $path"
        }
        return @{ ok = $true; removed = $removed }
    } catch {
        Add-Log "error" "file in use or unwritable: $path ($($_.Exception.Message))"
        return @{ ok = $false; removed = 0 }
    }
}

function Clear-VmOptionsFile([string]$path) {
    try {
        $lines = [System.IO.File]::ReadAllLines($path)
        $kept = @($lines | Where-Object {
            (-not $script:AgentLineRegex.IsMatch($_)) -and (-not $script:NameLineRegex.IsMatch($_))
        })
        $removed = $lines.Count - $kept.Count
        if ($removed -gt 0) {
            $list = New-Object System.Collections.Generic.List[string]
            foreach ($l in $kept) { $null = $list.Add($l) }
            [System.IO.File]::WriteAllLines($path, $list)
            Add-Log "info" "removed $removed agent line(s) from $path"
        }
        return @{ ok = $true; removed = $removed }
    } catch {
        Add-Log "error" "file in use or unwritable: $path ($($_.Exception.Message))"
        return @{ ok = $false; removed = 0 }
    }
}

# ---------------------------------------------------------------------------
# Environment variables (Remove_Env from ckey_script.ps1: User + Machine)
# ---------------------------------------------------------------------------
function Remove-ProductEnv([string]$prd) {
    $key = "$($prd.ToUpper())_VM_OPTIONS"
    $removed = @()
    foreach ($scope in @('User', 'Machine')) {
        try {
            $val = [Environment]::GetEnvironmentVariable($key, $scope)
            if (-not [string]::IsNullOrEmpty($val)) {
                [Environment]::SetEnvironmentVariable($key, $null, $scope)
                $removed += "$scope`:$key"
                Add-Log "info" "removed env $key ($scope scope)"
            }
        } catch {
            Add-Log "warn" "cannot remove $key from $scope scope (admin required?): $($_.Exception.Message)"
        }
    }
    return $removed
}

# ---------------------------------------------------------------------------
# WM_SETTINGCHANGE broadcast (same Add-Type as ckey_script.ps1 era fix)
# ---------------------------------------------------------------------------
function Broadcast-EnvChange {
    try {
        if (-not ('Win32Env.Native' -as [type])) {
            Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
namespace Win32Env {
    public class Native {
        [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
        public static extern IntPtr SendMessageTimeout(
            IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
            uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
    }
}
"@
        }
        $out = [UIntPtr]::Zero
        [Win32Env.Native]::SendMessageTimeout([IntPtr]0xffff, 0x1a, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$out) | Out-Null
        Add-Log "info" "WM_SETTINGCHANGE broadcast sent"
    } catch {
        Add-Log "warn" "broadcast failed: $($_.Exception.Message)"
    }
}

# ---------------------------------------------------------------------------
# Modes
# ---------------------------------------------------------------------------
# NOTE: PowerShell variables are CASE-INSENSITIVE and the [string]$Products
# parameter is TYPE-CONSTRAINED: assigning an array to a differently-cased
# `$products` would silently stringify the array with spaces. Use a dedicated
# unconstrained variable for the parsed list.
$productList = @()
$rawProducts = "$Products"
if ($rawProducts -and $rawProducts.Trim()) {
    $productList = @($rawProducts.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ })
}

$result = @{
    ok      = $true
    mode    = $Mode
    results = New-Object System.Collections.ArrayList
    logs    = $script:Logs
}

switch ($Mode) {

    'cleanup-env' {
        $removedAll = @()
        foreach ($prd in $productList) { $removedAll += (Remove-ProductEnv $prd) }
        Broadcast-EnvChange
        $null = $result.results.Add(@{
            product     = ($productList -join ',')
            ok          = $true
            removed_env = $removedAll
            edited      = @()
        })
    }

    'install' {
        if (-not $AgentJar -or -not (Test-Path -LiteralPath $AgentJar)) {
            $result.ok = $false
            Add-Log "error" "agent jar not found: $AgentJar"
            break
        }
        $agentSafe = Get-ShortPath $AgentJar
        Add-Log "debug" "agent jar (resolved): $agentSafe"
        if ($script:UnsafePathRegex.IsMatch($agentSafe)) {
            # 8.3 short path unavailable (disabled volume?) -> let Rust fall back
            # to copying the agent into a space-free directory and retry.
            $result.ok = $false
            Add-Log "error" "unsafe-agent-path: $agentSafe (space-free resolution failed)"
            break
        }
        $agentLine = "-javaagent:$($agentSafe.Replace('\','/'))=jetbrains"
        Add-Log "info" "agent line: $agentLine"

        foreach ($prd in $productList) {
            Add-Log "debug" "processing product: $prd"
            # 0. env cleanup first (ckey_script.ps1 removes before processing)
            $removedEnv = @(Remove-ProductEnv $prd)

            $locations = @(Find-ProductLocations $prd)
            $edited = @()
            $prdOk = $false

            foreach ($loc in $locations) {
                $targets = @($loc.binVmOptions) + @($loc.roamingVmOptions)
                foreach ($vm in $targets) {
                    if (-not $vm -or -not (Test-Path -LiteralPath $vm)) { continue }
                    $r = Repair-VmOptionsFile $vm $agentLine
                    if ($r.ok) { $edited += $vm; $prdOk = $true }
                }
            }

            if ($prdOk) {
                Add-Log "success" "installed for $prd -> $($edited -join ', ')"
            } else {
                Add-Log "warn" "no vmoptions found for $prd (IDE not detected), skipped"
            }

            $null = $result.results.Add(@{
                product     = $prd
                ok          = $prdOk
                edited      = $edited
                removed_env = $removedEnv
                agent_line  = $agentLine
            })
        }
        Broadcast-EnvChange
    }

    'uninstall' {
        foreach ($prd in $productList) {
            $removedEnv = @(Remove-ProductEnv $prd)
            $locations = @(Find-ProductLocations $prd)
            $edited = @()
            foreach ($loc in $locations) {
                $targets = @($loc.binVmOptions) + @($loc.roamingVmOptions)
                foreach ($vm in $targets) {
                    if (-not $vm -or -not (Test-Path -LiteralPath $vm)) { continue }
                    $r = Clear-VmOptionsFile $vm
                    if ($r.ok -and $r.removed -gt 0) { $edited += $vm }
                }
            }
            if ($edited.Count -gt 0) {
                Add-Log "success" "uninstalled for $prd -> $($edited -join ', ')"
            } else {
                Add-Log "info" "no agent lines found for $prd"
            }
            $null = $result.results.Add(@{
                product     = $prd
                ok          = $true
                edited      = $edited
                removed_env = $removedEnv
            })
        }
        Broadcast-EnvChange
    }

    default {
        $result.ok = $false
        Add-Log "error" "unknown mode: $Mode"
    }
}

$result.logs = $script:Logs
ConvertTo-Json -InputObject $result -Depth 6 -Compress
