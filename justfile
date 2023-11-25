shebang := if os() == 'windows' {
  'powershell.exe'
} else {
  '/usr/bin/bash'
}

default: druid
    
druid: 
    #!{{shebang}}
    cd druid
    cargo rr

dioxus:
    #!{{shebang}}
    cd dioxus
    cargo rr

iced:
    #!{{shebang}}
    cd iced
    cargo rr

egui:
    #!{{shebang}}
    cd egui
    cargo rr

slint:
    #!{{shebang}}
    cd slint
    cargo rr