# Node Hashing

In this section, we will describe how leaf secure key and node merkle hash are computed. 
As mentioned before, there's a hash function provided by the hash scheme, denoted as `h` in the doc below.

<aside>
💡 Note: We use `init_state = 0` in the Poseidon hash function for all use cases in the zkTrie.
</aside>

## Node Kinds

### Empty Node

The node hash of an empty node is 0.

### Branch Node

The branch node hash is computed as follows:
```
branch_node_hash = h(left_child_hash, right_child_hash)
```

### Leaf Node

The node hash of a leaf node is computed as follows:
```
leaf_node_hash = h(h(1, node_key), value_hash)
```

## Scroll Implementation

In scroll zkTrie, the Poseidon hash scheme is used.

### Key Hashing

For any key which length is less or equal to 32 bytes, the `node_key` is hashed by:
```
let mut v_lo = [0u8; HASH_SIZE];
let mut v_hi = [0u8; HASH_SIZE];
if v.len() > HALF_LEN {
    v_lo[HALF_LEN..].copy_from_slice(&v[..HALF_LEN]);
    v_hi[HALF_LEN..v.len()].copy_from_slice(&v[HALF_LEN..]);
} else {
    v_lo[HALF_LEN..HALF_LEN + v.len()].copy_from_slice(v);
}

node_key = h(v_hi, v_lo)
```

We denote `key_hash` as the function to compute the `node_key` from the key.

### Leaf Nodes

The leaf node can hold two types of values: Ethereum accounts and storage key-value pairs. 
Next, we will describe how the node key and value hash are computed for each leaf node type.

### Ethereum Account Leaf Node

For an Ethereum Account Leaf Node, it consists of an Ethereum address and a state account struct. 
The node key is derived from the Ethereum address:
```
node_key = key_hash(address)
```

A state account struct in the Scroll consists of the following fields 
(`Fr` indicates the finite field used in Poseidon hash and is a 254-bit value)

- `nonce`: u64
- `code_size`: u64
- `balance`: u256, but treated as Fr
- `storage_root`: Fr
- `keccak_code_hash`: H256
- `poseidon_code_hash`: Fr

Before computing the value hash, the state account is first encodes into a list of `[u8; 32]` values. 
The encoding scheme is:

```
(The following scheme assumes the big-endian encoding)
[0:32] (bytes in big-endian)
	[0:16] Reserved with all 0
	[16:24] code_size, u64 in big-endian
	[24:32] nonce, u64 in big-endian
[32:64] balance
[64:96] storage_root
[96:128] keccak_code_hash
[128:160] poseidon_code_hash
(total 160 bytes)
```

The marshal function also returns a `compression_flag` value along with the list of `[u8; 32]` values. 

The `compression_flag` is a bitmap that indicates whether a `[u8; 32]` value CANNOT be treated as a field element (Fr).
The `compression_flag` value for state account is 8, shown below.

```
+--------------------+---------+------+----------+----------+
|          0         |    1    |   2  |     3    |     4    | (index)
+--------------------+---------+------+----------+----------+
| nonce||codesize||0 | balance | root |  keccak  | poseidon | (u256)
+--------------------+---------+------+----------+----------+
|          0         |    0    |   0  |     1    |     0    | (flag bits)
+--------------------+---------+------+----------+----------+
(LSB)                                                   (MSB)
```

The value hash is computed in two steps:

1. Convert the value that cannot be represented as a field element of the Poseidon hash to the field element.
2. Combine field elements in a binary tree structure till the tree root is treated as the value hash.

In the first step, when the bit in the `compression_flag` is 1 indicating the `[u8; 32]` value that cannot be treated 
as a field element, we split the value into a high-128bit value and a low-128bit value, and then pass them to 
a Poseidon hash to derive a field element value, `h(value_hi, value_lo)`.

Based on the definition, the value hash of the state account is computed as follows:

```
value_hash =
h(
    h(
        h(nonce||codesize||0, balance),
        h(
            storage_root,
            h(keccak_code_hash[0:16], keccak_code_hash[16:32]), // convert Keccak codehash to a field element
        ),
    ),
    poseidon_code_hash,
)
```

#### Storage Leaf Node

For a Storage Leaf Node, it is a key-value pair, which both are a `[u8; 32]` value. 
The node key of this leaf node is derived from the storage key:
```
node_key = key_hash(storage_key)
```

The storage value is a `u256` value. The `flag` for the storage value is 1, showed below.

```
+-------+
|   0   | (index)
+-------+
| value | (u256)
+-------+
|   1   | (flag bits)
+-------+
```

The value hash is computed as follows:

```
value_hash = h(storage_value[0:16], storage_value[16:32])
```
