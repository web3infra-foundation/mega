# Decentralize Open Source Collaboration

## Decentralize Collaboration Event

```bash
{
  "kind": 111,
  "id": <32-bytes lowercase hex-encoded sha256 of the serialized event data>,
  "pubkey": <32-bytes lowercase hex-encoded public key of the event creator>,
  "created_at": <unix timestamp in seconds>,
  "tags": [
    ["uri", <p2p://<32-bytes lowercase hex-encoded public key of the event creator>/<repository name>.git>],
    ["action", <event type, include repo/issue/mr>, <repo: open/close/update; issue: open/comment/close; mr: open/comment/review/merge/close>],
    ["commit", <git hash id>],
    ["ref", <ref of the event, include branch/tag/issue/mr>],
    ["title", <arbitrary string>],
    ["content", <arbitrary string>],
    ["sig", <64-byte lowercase hex of the signature of the sha256 hash of the serialized event data, which is the same as the id field>]
  ],
  "content": <arbitrary string>,
  "sig": < 64-byte lowercase hex of the signature of the sha256 hash of the serialized event data, which is the same as the id field>
}
```

### Customization of the git P2P transfer protocol

The original Git protocol syntax is

```bash
[<protocol>://]<username>[:<password>]@<hostname>[:<port>]/<namespace>/<repo>[.git]
```

For a P2P Git protocol

- <protocol> could be a prefix like p2p:// to indicate the P2P protocol
- <username> and <password> is unnecessary for P2P protocol. In implementation. We reference the Git SSH protocol interaction commands and use the peer ID for authentication
- The <hostname> usually represents the server, but in P2P, it maps to the peer ID hosting the repo. We use <peerID> here to avoid confusion
- The <port>  will not be relevant for p2p networking
- The mega uses mono repo, so there are no <namespaces> or <repo> names, only <path>. We could design a virtual path scheme to map directories to exposed public paths privately

The Git version control system uses two major transfer protocols: "dumb" and "smart." The dumb protocol is simple but inefficient, requiring a series of HTTP GET requests. It is rarely used today due to its limitations in security and efficiency. On the other hand, the smart protocol is more common and efficient, as it allows for intelligent data transfer between the client and server.

Inspired by Git's approach to having multiple transfer protocols, we add a type segment in the custom P2P Git transport protocol. This segment allows us to specify the format of the files being transferred between peers, similar to how Git's protocols specify the nature of the data transfer. Currently, the type segment supports two formats: pack and object.

- `pack`: Indicates that the file being transferred is in Git's Pack format, efficiently transferring multiple Git objects.
- `object`: Indicates that the file being transferred is in Git's Object format, suitable for transferring individual Git objects like blobs, trees, commits, or tags.

Finally, the P2P protocol URI looks like

```bash
p2p://<peerId>/<type>/<repo>
```

Example

```bash
p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/pack/mega.git
```

or

```bash
p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/object/be044281f9604305e1b41b0e800e844c2a417e52
```

## Customization of the Git Peer-to-Peer Transfer Protocol