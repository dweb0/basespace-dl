// Simplified version of https://docs.rs/exitfailure, but plan on
// adding colors to "Error" tag
use std::fmt;

pub struct ExitFailure(failure::Error);

impl fmt::Debug for ExitFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fail = self.0.as_fail();
        write!(f, "{}", fail)?;

        for cause in fail.iter_causes() {
            write!(f, "\nInfo: caused by {}", cause)?;
        }

        if let Ok(x) = std::env::var("RUST_BACKTRACE") {
            if x != "0" {
                write!(f, "\n{}", self.0.backtrace())?
            }
        }
        Ok(())
    }
}

impl<T: Into<failure::Error>> From<T> for ExitFailure {
    fn from(t: T) -> Self {
        ExitFailure(t.into())
    }
}