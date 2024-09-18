# Introduction

In essence, zkTrie is a sparse binary Merkle Patricia Trie, depicted in the above figure.
Before diving into the Sparse Binary Merkle Patricia Trie, let's briefly touch on Merkle Trees and Patricia Tries.

- **Merkle Tree**: A Merkle Tree is a tree where each leaf node represents a hash of a data block, 
  and each non-leaf node represents the hash of its child nodes.
- **Patricia Trie**: A Patricia Trie is a type of radix tree or compressed trie used to 
  store key-value pairs efficiently. It encodes the nodes with same prefix of the key to share the common path, 
  where the path is determined by the value of the node key.
