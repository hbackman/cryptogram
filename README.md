# Cryptogram

A decentralized microblogging platform on blockchain.

## Overview

Cryptogram (TBD) is a distributed microblogging service built on a custom blockchain.
Each post is stored as a transaction on the blockchain, ensuring immutability,
censorship resistance, and decentralization. Nodes participate in the network
to store and validate posts, using a peer-to-peer (P2P) network for communication.

## Contributing

Contributions are welcome! Feel free to fork, submit PRs, or open issues.

## TODO

- Blockchain sync improvements - The blockchain sync simply downloads the entire
  blockchain from a random node. This node could lie and this would be bad. It
  can also only send the chain if it's small enough to fit in the MTU.
- Blockchain disk peristance improvements
- Posts
  - Size validation (300 chars)
  - Images?
  - Replies
  - Retweets
- Users
  - Size validation (255 chars?)
  - Images?
