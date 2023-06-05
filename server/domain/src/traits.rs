#[async_trait::async_trait]
pub trait Store: TypesStore + PartStore {}

mod types;
pub use types::*;

mod part;
pub use part::*;
