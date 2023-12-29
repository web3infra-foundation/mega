## How to use the p2p function

### start a relay-server

```
cargo run service p2p --host 0.0.0.0 --p2p-port 8200 --relay-server
or
cargo run service p2p --host 0.0.0.0 --p2p-port 8200 --relay-server
```

### start a client

```
cargo run service p2p --host 0.0.0.0 --p2p-port 8201 --bootstrap-node /ip4/{relay-server-ip}/tcp/8200 --secret-key 6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b7181
```

### start another client

```
cargo run service p2p --host 0.0.0.0 --p2p-port 8202 --bootstrap-node /ip4/{relay-server-ip}/tcp/8200
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

### try to clone a repository

```
mega clone p2p://12D3KooWPjceQrSwdWXPyLLeABRXmuqt69Rg3sBYbU1Nft9HyQ6X/mega_test.git
```

```
mega pull p2p://12D3KooWPjceQrSwdWXPyLLeABRXmuqt69Rg3sBYbU1Nft9HyQ6X/mega_test.git
```

### share a repository to DHT

```
mega provide mega_test.git
```

### clone git-object from p2p network

```
mega clone-object mega_test.git
```

### pull git-object from p2p network

```
mega pull-object mega_test.git
```
