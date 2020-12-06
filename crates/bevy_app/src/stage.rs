/// Name of the app stage that runs once at the beginning of the app
pub const STARTUP: &str = "startup";

/// Name of app stage that runs before all other app stages
pub const FIRST: &str = "first";

/// Name of app stage that runs before EVENT
pub const PRE_EVENT: &str = "pre_event";

/// Name of app stage that updates events. Runs before UPDATE
pub const EVENT: &str = "event";

/// Name of app stage responsible for performing setup before an update. Runs before UPDATE.
pub const PRE_UPDATE: &str = "pre_update";

/// Name of app stage responsible for doing most app logic. Systems should be registered here by default.
pub const UPDATE: &str = "update";

/// Name of app stage responsible for processing the results of UPDATE. Runs after UPDATE.
pub const POST_UPDATE: &str = "post_update";

/// Name of app stage that runs after all other app stages
pub const LAST: &str = "last";
