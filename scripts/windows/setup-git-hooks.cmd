@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-git-hooks.ps1" %*
