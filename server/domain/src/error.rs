/* 
    tendabike - the bike maintenance tracker
    
    Copyright (C) 2023  Christoph Rohland 

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.

 */

//! This module contains the error types used throughout the application.
//!
//! The `Error` enum defines the different types of errors that can occur, and the `AnyResult` type
//! is a convenient alias for `Result<T, anyhow::Error>`.
//!
use thiserror::Error;
pub use anyhow::{Context, ensure, bail};

#[derive(Clone, Debug, Error)]
pub enum Error{
    #[error("User not authenticated: {0}")]
    NotAuth(String),
    #[error("Forbidden request: {0}")]
    Forbidden(String),
    #[error("Object not found: {0}")]
    NotFound(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Try again: {0}")]
    TryAgain(&'static str),
}

pub type AnyResult<T> = Result<T,anyhow::Error>;