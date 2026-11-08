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

//! The provider-agnostic seams shared by every activity provider (spec #781).
//!
//! [`facade`] — [`DomainFacade`], the trait a provider goes through to reach
//! the domain, and its concrete implementation. A provider function is generic
//! over `D: DomainFacade` (static dispatch); tests swap in the
//! [`MockDomainFacade`] (under `test-support`).
//!
//! Nothing consumes the seam yet — this is pure foundation (ticket #782).

mod facade;
pub use facade::*;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
