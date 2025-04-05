# Rename MergeMeshError to MeshMergeError and add IncompatiblePrimitiveTopology variant

prs = [18561]

- Users will need to rename MergeMeshError to MeshMergeError
- When handling MergeMeshError (now MeshMergeError), users will need to account for the new IncompatiblePrimitiveTopology variant, as it has been changed from a struct to an enum
