
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

#[async_trait::async_trait]
pub trait Store: diesel_async::AsyncConnection + TypesStore + PartStore + UserStore + ActivityStore + AttachmentStore  {
}