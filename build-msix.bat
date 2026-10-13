@echo off
setlocal EnableDelayedExpansion
rem Double-click to build the Microsoft Store package (MSIX).
rem Not set up with Partner Center yet? It offers a test package you can install on this PC.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-msix.ps1" %*
if errorlevel 1 (
  echo.
  set /p T=Build a test package to try on this PC instead? [y/N] 
  if /i "!T!"=="y" powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-msix.ps1" -Test
)
pause
