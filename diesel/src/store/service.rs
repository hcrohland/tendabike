use crate::AsyncDieselConn;

#[async_session::async_trait]
impl tb_domain::ServiceStore for AsyncDieselConn {}
