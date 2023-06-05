
mod types;
pub use types::*;

#[async_trait::async_trait]
pub trait Store: TypesStore + PartStore {}

pub use parts::*;
mod parts;
