//! Error codes used by BRP.

// JSON-RPC errors
// Note that the range -32728 to -32000 (inclusive) is reserved by the JSON-RPC specification.

/// Invalid JSON.
pub const PARSE_ERROR: i16 = -32700;

/// JSON sent is not a valid request object.
pub const INVALID_REQUEST: i16 = -32600;

/// The method does not exist / is not available.
pub const METHOD_NOT_FOUND: i16 = -32601;

/// Invalid method parameter(s).
pub const INVALID_PARAMS: i16 = -32602;

/// Internal error.
pub const INTERNAL_ERROR: i16 = -32603;

// Bevy errors (i.e. application errors)

/// Entity not found.
pub const ENTITY_NOT_FOUND: i16 = -23401;

/// Could not reflect or find component.
pub const COMPONENT_ERROR: i16 = -23402;

/// Could not find component in entity.
pub const COMPONENT_NOT_PRESENT: i16 = -23403;

/// Cannot reparent an entity to itself.
pub const SELF_REPARENT: i16 = -23404;
