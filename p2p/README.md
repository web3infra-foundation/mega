## How to use the p2p function

### start a relay-server

```
cargo run p2p --host 0.0.0.0 --port 8001 --relay-server
```

### start a client

```
cargo run p2p --host 0.0.0.0 --port 8002 --bootstrap-node /ip4/{relay-server-ip}/tcp/8001
```

### start another client

```
cargo run p2p --host 0.0.0.0 --port 8003 --bootstrap-node /ip4/{relay-server-ip}/tcp/8001
```

### try to use DHT

#### put a key-value to p2p network in one terminal

```
kad put 123 abc 
```

#### get a key-value from p2p network in another terminal

```
kad get 123
```

### try to share a file

#### in one client terminal

```
file provide aaa.txt README.md
```

#### in another client terminal

```
file get aaa.txt
```
