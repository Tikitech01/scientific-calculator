; Inno Setup script for the Scientific Calculator installer. Built by build-windows.ps1.
#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif

[Setup]
AppId={{6B1D2C5E-7A44-4F0B-9C3E-4E5A1B7D0A21}
AppName=Scientific Calculator
AppVersion={#AppVersion}
AppPublisher=Sopè Technology
AppPublisherURL=https://sopetechnology.com
AppCopyright=(c) 2026 Sopè Technology
DefaultDirName={autopf}\Scientific Calculator
DefaultGroupName=Scientific Calculator
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=Scientific-Calculator-{#AppVersion}-Windows-Setup
SetupIconFile=icon.ico
UninstallDisplayIcon={app}\ScientificCalculator.exe
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
Source: "..\target\release\scientific-calculator.exe"; DestDir: "{app}"; DestName: "ScientificCalculator.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Scientific Calculator"; Filename: "{app}\ScientificCalculator.exe"
Name: "{autodesktop}\Scientific Calculator"; Filename: "{app}\ScientificCalculator.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\ScientificCalculator.exe"; Description: "Open Scientific Calculator"; Flags: nowait postinstall skipifsilent
