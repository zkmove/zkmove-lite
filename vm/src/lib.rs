// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::clone_on_copy)]
#![allow(clippy::from_over_into)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::manual_map)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::new_without_default)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::single_match)]

pub mod chips;
pub mod circuit;
pub mod frame;
pub mod interpreter;
pub mod locals;
pub mod program_block;
pub mod runtime;
pub mod stack;
pub mod value;
