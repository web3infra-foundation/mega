gateway_ip=$(ip route | grep default | awk '{print $3}')

if [ -z "$gateway_ip" ]; then
    echo "Unable to find the default gateway."
    exit 1
fi

# BUCK_PROJECT_ROOT="/home/bean/projects/buck2"
# SERVER_WS="ws://localhost:8004/ws"

export BUCK_PROJECT_ROOT="/opt/megadir/mount" # TODO: need to cooperate with the scprio
export SERVER_WS="ws://$gateway_ip:8004/ws"

exec /root/.cargo/bin/orion