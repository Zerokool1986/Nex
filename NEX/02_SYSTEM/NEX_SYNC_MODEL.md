# NEX Anti-Entropy & Synchronization Model

Nodes reconcile state by exchanging Sparse Merkle Tree (SMT) root hashes. When roots differ, nodes traverse the tree levels to identify missing sub-branches and fetch mutations.
