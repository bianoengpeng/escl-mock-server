use std::fmt::{Display, Formatter};

pub(crate) struct ScanJob {
    pub retrieved_pages: u32,
}

impl Default for ScanJob {
    fn default() -> Self {
        ScanJob {
            retrieved_pages: 0
        }
    }
}

impl Display for ScanJob {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "retrieved_pages = {}", self.retrieved_pages)
    }
}