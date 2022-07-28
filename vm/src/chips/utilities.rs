// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::NUM_OF_ADVICE_COLUMNS;
use crate::value::Value;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Region},
    plonk::{Advice, Column, Error, Expression, VirtualCells},
    poly::Rotation,
};
use logger::prelude::*;
use std::convert::TryInto;
use std::marker::PhantomData;

pub const NUM_OF_BYTES_U128: usize = 16;

#[derive(Clone, Debug)]
pub struct Cell<F: FieldExt> {
    pub expression: Expression<F>,
    pub column: Column<Advice>,
    pub rotation: Rotation,
}

impl<F: FieldExt> Cell<F> {
    pub fn new(meta: &mut VirtualCells<F>, column: Column<Advice>, rotation: i32) -> Self {
        Cell {
            expression: meta.query_advice(column, Rotation(rotation)),
            column,
            rotation: Rotation(rotation),
        }
    }

    pub fn assign(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        value: Option<F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        region.assign_advice(
            || "assign cell",
            self.column,
            (offset as i32 + self.rotation.0) as usize,
            || {
                value.ok_or_else(|| {
                    error!("assigned value is None");
                    Error::Synthesis
                })
            },
        )
    }
}

pub(crate) trait Expr<F: FieldExt> {
    fn expr(&self) -> Expression<F>;
}

impl<F: FieldExt> Expr<F> for u64 {
    fn expr(&self) -> Expression<F> {
        Expression::Constant(F::from(*self))
    }
}

/// The internal representation of FieldExt is four 64-bits unsigned integer in
/// little-endian order, this struct has 32 Cells, to hold the 32 bytes of the
/// internal representation of the field element.
#[derive(Clone, Debug)]
pub struct ByteRepresentation<F: FieldExt>(pub(crate) [Cell<F>; 32]);

impl<F: FieldExt> ByteRepresentation<F> {
    pub fn construct(
        meta: &mut VirtualCells<F>,
        advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
        offset: usize,
    ) -> Self {
        let mut cells = Vec::new();
        for i in 0..32 {
            let column_index = i % NUM_OF_ADVICE_COLUMNS;
            let rotation = i / NUM_OF_ADVICE_COLUMNS + offset;
            cells.push(Cell::new(meta, advices[column_index], rotation as i32))
        }
        cells.into()
    }

    pub fn lower_16_bytes_expr(&self) -> Expression<F> {
        let mut value = 0.expr();
        let mut multiplier = F::one();

        for byte in self.0.iter().take(16) {
            value = value + byte.expression.clone() * multiplier;
            multiplier *= F::from(256);
        }
        value
    }
}

impl<F: FieldExt> From<Vec<Cell<F>>> for ByteRepresentation<F> {
    fn from(bytes: Vec<Cell<F>>) -> ByteRepresentation<F> {
        let bytes: [Cell<F>; 32] = bytes.try_into().unwrap_or_else(|v: Vec<Cell<F>>| {
            panic!("Expected a Vec of length {} but it was {}", 32, v.len())
        });
        ByteRepresentation(bytes)
    }
}

pub struct DiffBytes<F: FieldExt> {
    _marker: PhantomData<F>,
}

impl<F: FieldExt> DiffBytes<F> {
    /// if a >= b then diff = a - b; if a < b then diff = a - b + range;
    /// Move doesn't support u256, range 0~2^128 is workable for u8, u64, u128
    /// convert diff into the byte representation, assign them into specified cells.
    pub fn assign_diff_bytes(
        region: &mut Region<'_, F>,
        cells: &ByteRepresentation<F>,
        a: Value<F>,
        b: Value<F>,
    ) -> Result<(), Error> {
        let lhs = a.value().ok_or_else(|| {
            error!("a.value is None");
            Error::Synthesis
        })?;
        let rhs = b.value().ok_or_else(|| {
            error!("b.value is None");
            Error::Synthesis
        })?;
        let range = F::from(2).pow(&[(NUM_OF_BYTES_U128 * 8) as u64, 0, 0, 0]);
        let range_or_zero = if lhs < rhs { range } else { F::zero() };
        let diff = (lhs - rhs) + range_or_zero;

        let diff_bytes: [u8; 32] = diff
            .to_repr()
            .as_ref()
            .try_into()
            .expect("Field fits into 256 bits");

        for (index, byte) in cells.0.iter().enumerate() {
            byte.assign(region, 0, Some(F::from(diff_bytes[index] as u64)))?;
        }

        Ok(())
    }
}
