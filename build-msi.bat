@echo off
rem Double-click to build the Windows .msi installer.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-msi.ps1"
pause
