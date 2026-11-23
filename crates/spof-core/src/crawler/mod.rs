pub mod ajax;
pub mod assignment_parser;
pub mod parser;
pub mod resource;
pub mod sync;

pub use assignment_parser::{parse_assignment_details, AssignmentDetails};
pub use sync::{default_cache_dir, CourseCrawler, SyncSummary};
