pub mod ajax;
pub mod parser;
pub mod resource;
pub mod sync;

pub use parser::AssignmentDetails;
pub use sync::{CourseCrawler, SyncSummary};
