# Tree Construction

Given a key-value pair, we first compute a *secure key* for the corresponding leaf node by hashing the original key
(i.e., account address and storage key) using the Poseidon hash function.
This can make the key uniformly distributed over the key space. The node key hashing method is described in the
[Node Hashing](./node_hashing.md) section below.

We then encode the path of a new leaf node by traversing the secure key from Least Significant Bit (LSB) 
to the Most Significant Bit (MSB). At each step, if the bit is 0, we will traverse to the left child; 
otherwise, traverse to the right child.

We limit the maximum depth of zkTrie to 248, meaning that the tree will only traverse the lower 248 bits of the key. 
This is because the secure key space is a finite field used by Poseidon hash that doesn't occupy the full range of
power of 2. This leads to an ambiguous bit representation of the key in a finite field and thus causes a soundness 
issue in the zk circuit. But if we truncate the key to lower 248 bits, the key space can fully occupy the range 
of $2^{248}$ and won't have the ambiguity in the bit representation.

We also apply an optimization to reduce the tree depth by contracting a subtree that has only one leaf node to a 
single leaf node. For example, in the Figure 1, the tree has three nodes in total, with keys `0100`, `0010`, and `1010`.
Because there is only one node that has a key with suffix `00`, the leaf node for key `0100` only traverses the 
suffix `00` and doesn't fully expand its key which would have resulted in depth of 4.
