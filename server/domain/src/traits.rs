#[async_trait::async_trait]
pub trait Store: TypesStore + PartStore + UserStore + ActivityStore + AttachmentStore  {}

mod types;
pub use types::*;

mod part;
pub use part::*;

mod user;
pub use user::*;

mod activity;
pub use activity::*;

mod attachment;
pub use attachment::*;
