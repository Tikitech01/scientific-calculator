@echo off
rem Double-click to build the Windows installer.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-windows.ps1"
pause
