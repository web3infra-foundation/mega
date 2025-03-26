[[ -z $DISPLAY && $XDG_VTNR -eq 1 ]] && sh -c "archinstall --config user_configuration.json --creds user_credentials.json --disk_layout user_disk_layout.json"
