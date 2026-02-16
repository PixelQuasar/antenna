pub struct Logger;

impl Logger {
    fn log_msg(msg: &str) -> String {
        format!("[ANTENNA LOG] {}", msg)
    }

    pub fn warn_msg(msg: &str) -> String {
        Self::log_msg(&format!("WARNING: {}", msg))
    }

    pub fn _error_msg(msg: &str) -> String {
        Self::log_msg(&format!("ERROR: {}", msg))
    }

    pub fn _debug_msg(msg: &str) -> String {
        Self::log_msg(&format!("DEBUG: {}", msg))
    }
}
