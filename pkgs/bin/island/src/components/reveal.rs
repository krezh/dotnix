//! Things coming into view and going out of it: rising a little as they fade in, and sliding
//! aside as they fade out, the way a pop-up card does.

use std::time::Duration;

/// How long [`leave`] takes, for whatever keeps the element around until it is gone.
pub const LEAVE: Duration = Duration::from_millis(220);
