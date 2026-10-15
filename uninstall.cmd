@echo off
rem Removes iw-helper and what it leaves on the PC: see uninstall.ps1.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0uninstall.ps1"
