@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0ci-check.ps1" %*
