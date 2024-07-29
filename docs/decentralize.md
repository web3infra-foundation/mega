# Decentralize Open Source Collaboration

## Decentralize Collaboration Event

```json
{
  "kind": 111,
  "id": <32-bytes lowercase hex-encoded sha256 of the serialized event data>,
  "peer": <32-bytes lowercase hex-encoded public key of the event creator>,
  "timestamp": <unix timestamp in seconds>,
  "tags": [
    
  ],
  "content": <arbitrary string>,
  "sig": < 64-byte lowercase hex of the signature of the sha256 hash of the serialized event data, which is the same as the "id" field>
}


```

## Customization of the Git Peer-to-Peer Transfer Protocol