; L'installation Windows de BPM-Caddy (Inno Setup 6).
;
; Elle ne pose que le **lanceur** : c'est lui qui va chercher l'application
; à la publication la plus récente, la tient à jour et la lance — comme
; quand on le télécharge seul. Ce que l'installation ajoute : une place
; stable pour le lanceur (le lancement à l'ouverture de session le nomme ;
; laissé dans « Téléchargements », il disparaissait au premier ménage),
; une entrée au menu Démarrer, une icône sur le bureau au choix, et une
; désinstallation propre.
;
; **Par utilisateur, sans droits d'administrateur** : un poste d'officine
; n'en a souvent pas, et l'application vit déjà dans le profil.
;
; **La désinstallation ne touche jamais à la base**, ni à config.toml : elle
; retire le lanceur, l'application téléchargée et l'entrée de démarrage.
;
; Construit par .github/workflows/release.yml :
;   iscc /DAppVersion=0.435.0 /DLauncher=chemin\du\lanceur.exe installer\bpm-caddy.iss

#ifndef AppVersion
  #define AppVersion "0"
#endif
#ifndef Launcher
  #define Launcher "..\bpm-caddy-launcher-windows-x86_64.exe"
#endif

[Setup]
AppId={{8611EEA1-CF78-4188-B1BE-044195E738D0}
AppName=BPM-Caddy
AppVersion={#AppVersion}
AppVerName=BPM-Caddy
AppPublisher=BPM-Caddy
AppPublisherURL=https://github.com/youssefsahli/bpm-caddy
DefaultDirName={localappdata}\Programs\BPM-Caddy
DefaultGroupName=BPM-Caddy
DisableProgramGroupPage=yes
DisableDirPage=yes
DisableReadyPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist-installer
OutputBaseFilename=bpm-caddy-setup-windows-x86_64
SetupIconFile=..\assets\bpm-caddy.ico
UninstallDisplayIcon={app}\bpm-caddy-launcher.exe
UninstallDisplayName=BPM-Caddy
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#Launcher}"; DestDir: "{app}"; DestName: "bpm-caddy-launcher.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\BPM-Caddy"; Filename: "{app}\bpm-caddy-launcher.exe"
Name: "{autodesktop}\BPM-Caddy"; Filename: "{app}\bpm-caddy-launcher.exe"; Tasks: desktopicon

[Registry]
; L'entrée « BPM-Caddy » du lancement à l'ouverture de session, que
; l'application écrit elle-même (Options › Base) : rien n'est créé ici,
; elle est seulement retirée à la désinstallation.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "BPM-Caddy"; Flags: uninsdeletevalue dontcreatekey

[UninstallDelete]
; Ce que le lanceur laisse en se remplaçant lui-même (l'ancien, de côté).
Type: files; Name: "{app}\bpm-caddy-launcher.exe.old"
Type: files; Name: "{app}\bpm-caddy-launcher.exe.*.nouveau"
; Ce que le lanceur a téléchargé à côté de la configuration — l'application
; et ses deux témoins de version. Ni la base, ni config.toml, ni les copies.
Type: files; Name: "{userappdata}\bpm-caddy\bpm-caddy.exe"
Type: files; Name: "{userappdata}\bpm-caddy\version.txt"
Type: files; Name: "{userappdata}\bpm-caddy\version.size"

[Run]
Filename: "{app}\bpm-caddy-launcher.exe"; Description: "{cm:LaunchProgram,BPM-Caddy}"; Flags: nowait postinstall skipifsilent
