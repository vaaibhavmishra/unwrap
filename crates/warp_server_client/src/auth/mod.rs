pub use user_uid::{TEST_USER_EMAIL, TEST_USER_UID, UserUid};
pub use warp_server_auth::user_uid;

/// Header used by legacy request metadata.
pub const EXPERIMENT_ID_HEADER: &str = "X-Warp-Experiment-Id";
