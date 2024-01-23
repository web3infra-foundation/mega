## How to use the p2p function

### start a relay-server

```
cargo run service p2p --host 0.0.0.0 --p2p-port 8200 --relay-server
or
cargo run service p2p --host 0.0.0.0 --p2p-port 8200 --relay-server --secret-key 6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b7180
```

### start a client

```
cargo run service p2p --host 0.0.0.0 --p2p-port 8201 --bootstrap-node /ip4/{relay-server-ip}/tcp/8200 --secret-key 6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b7181
```

### start another client

```
cargo run service p2p --host 0.0.0.0 --p2p-port 8202 --bootstrap-node /ip4/{relay-server-ip}/tcp/8200 --secret-key 6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b7182 --p2p-http-port 8002
```
