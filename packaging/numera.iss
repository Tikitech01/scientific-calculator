; Inno Setup script for the Numera installer. Built by build-windows.ps1.
#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif

[Setup]
AppId={{6B1D2C5E-7A44-4F0B-9C3E-4E5A1B7D0A21}
AppName=Numera
AppVersion={#AppVersion}
AppPublisher=Tiki Tech
DefaultDirName={autopf}\Numera
DefaultGroupName=Numera
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=Numera-{#AppVersion}-Setup
SetupIconFile=icon.ico
UninstallDisplayIcon={app}\Numera.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; Flags: unchecked

[Files]
Source: "..\target\release\numera.exe"; DestDir: "{app}"; DestName: "Numera.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Numera"; Filename: "{app}\Numera.exe"
Name: "{autodesktop}\Numera"; Filename: "{app}\Numera.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Numera.exe"; Description: "Open Numera"; Flags: nowait postinstall skipifsilent
