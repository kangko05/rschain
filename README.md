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
project/
├── network/            # P2P 네트워크 모듈
│   ├── mod.rs
│   ├── node.rs         # DHT 노드 구현
│   ├── message.rs      # 네트워크 메시지 정의
│   ├── kbucket.rs      # k-bucket 구현
│   └── operations.rs   # 네트워크 작업(읽기/쓰기)
├── blockchain/         # 블록체인 구현 모듈
│   ├── mod.rs
│   ├── block.rs        # 블록 구조체 정의
│   ├── transaction.rs  # 트랜잭션 구조체 정의
│   ├── chain.rs        # 블록체인 관리
│   ├── mempool.rs      # 메모리 풀 구현
│   └── consensus.rs    # 합의 알고리즘
└── node/               # 노드 유형 구현 모듈
    ├── mod.rs
    ├── bootstrap.rs    # 부트스트랩 노드
    ├── full.rs         # 풀 노드
    ├── mining.rs       # 마이닝 노드
    └── light.rs        # 라이트 노드
```
