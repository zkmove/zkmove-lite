// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

pub mod _mod;
pub mod add;
pub mod and;
pub mod common;
pub mod div;
pub mod eq;
pub mod lt;
pub mod mul;
pub mod neq;
pub mod not;
pub mod or;
pub mod sub;

pub enum Opcode {
    LdU8,
    LdU64,
    LdU128,
    Pop,
    Ret,
    Add,
    Mul,
    CopyLoc,
    Sub,
    Div,
    Mod,
    LdTrue,
    LdFalse,
    Eq,
    Neq,
    And,
    Or,
    Not,
    MoveLoc,
    StLoc,
    Branch,
    BrTrue,
    BrFalse,
    Call,
    Abort,
    Lt,
}
