## Gateway Module

The Gateway module serves as the primary handler for various requests including Git's clone, push, and pull operations, git-lfs client interactions, and web UI requests. While the majority of these requests are facilitated via the HTTP protocol, the Gateway module is also equipped to process Git requests through the SSH protocol.

