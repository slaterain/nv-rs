@echo off
rem Installs mods from the mods folder (.7z, .zip, .fomod), asking each installer's questions.
rem Your game folder is never changed. Mods go to userdata\mod installs.
start "" "%~dp0Play.exe" --install-mods
