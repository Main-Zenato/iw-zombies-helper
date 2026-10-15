# Removes iw-helper and everything it leaves on the PC. Start it with uninstall.cmd (double-click).
#
# What is removed:
# - the tool's files in this folder (programs, configuration, README, license, logs), then the folder
#   itself if nothing else is left in it. From a source checkout (Cargo.toml present), only logs\;
# - the interface's data (WebView2 cache) in %LOCALAPPDATA%\fr.zen.iw-helper;
# - on request, the two Windows graphics settings the tool can apply to the game.
#
# Nothing else is changed on the PC. The game's memory is never saved by the tool: nothing to undo there.

$ErrorActionPreference = 'Stop'
$fr = (Get-UICulture).TwoLetterISOLanguageName -eq 'fr'
function T([string]$en, [string]$french) { if ($fr) { $french } else { $en } }
function Ask([string]$question) {
    $answer = Read-Host "$question $(T '[y/N]' '[o/N]')"
    return $answer -match '^\s*(y|yes|o|oui)\s*$'
}
function Done { Read-Host (T 'Press Enter to close' 'Appuyez sur Entrée pour fermer') | Out-Null }

$here = $PSScriptRoot
$source = Test-Path (Join-Path $here 'Cargo.toml')
$files = 'iw-helper-gui.exe', 'iw-helper.exe', 'iw-helper.toml', 'README.md', 'LICENSE', 'uninstall.ps1'
$webview = Join-Path $env:LOCALAPPDATA 'fr.zen.iw-helper'
$gpuKey = 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences'
$ourSettings = 'GpuPreference', 'SwapEffectUpgradeEnable'

Write-Host ''
Write-Host (T 'iw-helper uninstaller' 'Désinstallation de iw-helper')
Write-Host ''

$running = Get-Process -Name 'iw-helper-gui', 'iw-helper', 'iw7_ship', 'iw7-mod' -ErrorAction SilentlyContinue
if ($running) {
    Write-Host (T 'Close the game and iw-helper first, then start again.' "Fermez d'abord le jeu et iw-helper, puis relancez.")
    Write-Host "  $(($running | Select-Object -ExpandProperty ProcessName -Unique) -join ', ')"
    Done; exit 1
}

# What is there to remove.
$targets = @()
if ($source) {
    $targets += Join-Path $here 'logs'
} else {
    $targets += $files | ForEach-Object { Join-Path $here $_ }
    $targets += Join-Path $here 'logs'
}
$targets = @($targets | Where-Object { Test-Path $_ })
if (Test-Path $webview) { $targets += $webview }

# Graphics settings the tool may have applied (Settings page), one registry value per game executable.
$gpuValues = @()
if (Test-Path $gpuKey) {
    $values = Get-ItemProperty $gpuKey
    foreach ($property in $values.PSObject.Properties) {
        if ($property.Name -like '*\iw7_ship.exe' -and "$($property.Value)" -match '(GpuPreference|SwapEffectUpgradeEnable)=') {
            $gpuValues += $property
        }
    }
}

if ($targets.Count -eq 0 -and $gpuValues.Count -eq 0) {
    Write-Host (T 'Nothing to remove: iw-helper is not installed here.' "Rien à supprimer : iw-helper n'est pas installé ici.")
    Done; exit 0
}

Write-Host (T 'This will be deleted:' 'Sera supprimé :')
$targets | ForEach-Object { Write-Host "  $_" }
if ($source) {
    Write-Host (T '  (source checkout: the code is kept, delete the folder yourself if you want)' "  (dossier source : le code est gardé, supprimez le dossier vous-même si vous voulez)")
}
Write-Host ''
if (-not (Ask (T 'Continue?' 'Continuer ?'))) { Write-Host (T 'Cancelled, nothing was changed.' "Annulé, rien n'a été modifié."); Done; exit 0 }

foreach ($target in $targets) {
    try {
        Remove-Item -LiteralPath $target -Recurse -Force
        Write-Host "  $(T 'deleted' 'supprimé') : $target"
    } catch {
        Write-Host "  $(T 'NOT deleted' 'NON supprimé') : $target ($($_.Exception.Message))"
    }
}

if ($gpuValues.Count -gt 0) {
    Write-Host ''
    Write-Host (T 'Windows graphics settings for the game (dedicated graphics card, windowed optimizations off):' 'Réglages graphiques Windows du jeu (carte graphique dédiée, optimisations fenêtrées coupées) :')
    $gpuValues | ForEach-Object { Write-Host "  $($_.Name) = $($_.Value)" }
    Write-Host (T 'They help the game run well. Remove them only if you want Windows defaults back.' 'Ils aident le jeu à bien tourner. Ne les retirez que pour revenir aux réglages de Windows.')
    if (Ask (T 'Remove them?' 'Les retirer ?')) {
        foreach ($value in $gpuValues) {
            # Keep any other option Windows stores in the same value.
            $kept = ("$($value.Value)" -split ';' | Where-Object { $_ -and ($ourSettings -notcontains ($_ -split '=')[0].Trim()) }) -join ';'
            if ($kept) {
                Set-ItemProperty -Path $gpuKey -Name $value.Name -Value "$kept;"
            } else {
                Remove-ItemProperty -Path $gpuKey -Name $value.Name
            }
            Write-Host "  $(T 'removed' 'retiré') : $($value.Name)"
        }
    }
}

Write-Host ''
Write-Host (T 'Last step, by hand: in Steam, open the game''s Properties and clear its launch options' 'Dernière étape, à la main : dans Steam, ouvrez les Propriétés du jeu et videz ses options de lancement')
Write-Host (T '(the line that starts iw-helper-gui.exe), otherwise the game will not start.' '(la ligne qui lance iw-helper-gui.exe), sinon le jeu ne démarrera plus.')
Write-Host (T 'Changes made to your game profile or account (Lobby and Account pages) stay: they are part of the game save.' 'Les changements faits au profil ou au compte du jeu (pages Salon et Compte) restent : ils font partie de la sauvegarde du jeu.')
Write-Host ''
Done

# Last: the launcher and the folder, once this window is closed (a running batch file cannot delete itself).
if (-not $source) {
    $launcher = Join-Path $here 'uninstall.cmd'
    $cleanup = "timeout /t 2 /nobreak >nul & del /q `"$launcher`" & rmdir `"$here`""
    Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', $cleanup -WorkingDirectory $env:TEMP -WindowStyle Hidden
}
