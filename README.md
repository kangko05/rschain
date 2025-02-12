### RSChain

RSChain is an experimental blockchain project implemented in Rust. The project aims to demonstrate blockchain fundamentals through a practical implementation and explore node interactions in a distributed environment.
Goals

### Implement basic blockchain architecture

1. Create P2P network using Gossip protocol
2. Build multi-node environment with Docker
3. Understand real-world blockchain mechanisms and challenges

# Tech Stack

Language: Rust
Containerization: Docker
Network Protocol: Gossip Protocol
Consensus Algorithm: PoW (Proof of Work)

### Core Features

1. Block creation and mining
2. P2P node communication
3. Transaction processing and propagation
4. Blockchain state synchronization
5. Peer discovery

```plain
rschain/
├── src/
│   ├── block/      # Block implementations
│   ├── chain/      # Blockchain structure
│   ├── network/    # P2P network implementation
│   ├── consensus/  # Consensus algorithms
│   └── main.rs
├── Dockerfile
├── docker-compose.yml
└── README.md
```
