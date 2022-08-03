// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::NUM_OF_ADVICE_COLUMNS;
use crate::value::Value;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Selector, VirtualCells},
    poly::Rotation,
};
use logger::prelude::*;
use std::convert::TryInto;

pub const NUM_OF_BYTES_U8: usize = 1;
pub const NUM_OF_BYTES_U64: usize = 8;
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
/// little-endian order, this struct has NUM_OF_BYTES Cells, to hold the lower
/// NUM_OF_BYTES bytes of the internal representation of the field element.
#[derive(Clone, Debug)]
pub struct ByteRepresentation<F: FieldExt, const NUM_OF_BYTES: usize>(
    pub(crate) [Cell<F>; NUM_OF_BYTES],
);

impl<F: FieldExt, const NUM_OF_BYTES: usize> ByteRepresentation<F, NUM_OF_BYTES> {
    pub fn construct(
        meta: &mut VirtualCells<F>,
        advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
        offset: usize,
    ) -> Self {
        let mut cells = Vec::new();
        for i in 0..NUM_OF_BYTES {
            let column_index = i % NUM_OF_ADVICE_COLUMNS;
            let rotation = i / NUM_OF_ADVICE_COLUMNS + offset;
            cells.push(Cell::new(meta, advices[column_index], rotation as i32))
        }
        cells.into()
    }

    pub fn assign(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        value: Option<F>,
    ) -> Result<(), Error> {
        match value {
            Some(value) => {
                let bytes: [u8; 32] = value
                    .to_repr()
                    .as_ref()
                    .try_into()
                    .expect("Field fits into 256 bits");

                for (index, cell) in self.0.iter().enumerate() {
                    cell.assign(region, offset, Some(F::from(bytes[index] as u64)))?;
                }
            }
            None => {
                for (_index, cell) in self.0.iter().enumerate() {
                    cell.assign(region, offset, None)?;
                }
            }
        }

        Ok(())
    }
}

impl<F: FieldExt, const NUM_OF_BYTES: usize> Expr<F> for ByteRepresentation<F, NUM_OF_BYTES> {
    fn expr(&self) -> Expression<F> {
        let mut value = 0.expr();
        let mut multiplier = F::one();

        for byte in self.0.iter() {
            value = value + byte.expression.clone() * multiplier;
            multiplier *= F::from(256);
        }
        value
    }
}

impl<F: FieldExt, const NUM_OF_BYTES: usize> From<Vec<Cell<F>>>
    for ByteRepresentation<F, NUM_OF_BYTES>
{
    fn from(bytes: Vec<Cell<F>>) -> ByteRepresentation<F, NUM_OF_BYTES> {
        let bytes: [Cell<F>; NUM_OF_BYTES] = bytes.try_into().unwrap_or_else(|v: Vec<Cell<F>>| {
            panic!(
                "Expected a Vec of length {} but it was {}",
                NUM_OF_BYTES,
                v.len()
            )
        });
        ByteRepresentation(bytes)
    }
}

/// Reconstruct a value from the input value's byte representation, if the
/// reconstructed value equals to the input value, then the input value is
/// in the given range.
#[derive(Clone, Debug)]
pub struct RangeCheckConfig<F: FieldExt, const NUM_OF_BYTES: usize> {
    pub(crate) s_range: Selector,
    pub(crate) cond_cell: Cell<F>,
    pub(crate) value_cell: Cell<F>,
    pub(crate) cells: ByteRepresentation<F, NUM_OF_BYTES>,
}

pub struct RangeCheckChip<F: FieldExt, const NUM_OF_BYTES: usize> {
    config: RangeCheckConfig<F, NUM_OF_BYTES>,
}

impl<F: FieldExt, const NUM_OF_BYTES: usize> RangeCheckChip<F, NUM_OF_BYTES> {
    pub fn construct(config: RangeCheckConfig<F, NUM_OF_BYTES>) -> Self {
        Self { config }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    ) -> RangeCheckConfig<F, NUM_OF_BYTES> {
        let mut cells = None;
        let mut cond_cell = None;
        let mut value_cell = None;

        let s_range = meta.selector();
        meta.create_gate("range check", |meta| {
            let cond = Cell::new(meta, advices[0], Rotation::cur().0 as i32);
            cond_cell = Some(cond.clone());
            let value = Cell::new(meta, advices[1], Rotation::cur().0 as i32);
            value_cell = Some(value.clone());
            let bytes = ByteRepresentation::construct(meta, advices, Rotation::next().0 as usize);
            cells = Some(bytes.clone());

            let s_range = meta.query_selector(s_range) * cond.expression;
            vec![s_range * (bytes.expr() - value.expression)]
        });

        RangeCheckConfig {
            s_range,
            cond_cell: cond_cell.unwrap(),
            value_cell: value_cell.unwrap(),
            cells: cells.unwrap(),
        }
    }

    pub fn assign(
        &self,
        layouter: &mut impl Layouter<F>,
        input_value: Value<F>,
        cond: Option<F>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "range check",
            |mut region: Region<'_, F>| {
                self.config.s_range.enable(&mut region, 0)?;
                self.config.cond_cell.assign(&mut region, 0, cond)?;
                let value =
                    self.config
                        .value_cell
                        .assign(&mut region, 0, input_value.value().clone())?;
                region
                    .constrain_equal(input_value.cell().ok_or(Error::Synthesis)?, value.cell())?;
                self.config
                    .cells
                    .assign(&mut region, 0, input_value.value())?;
                Ok(())
            },
        )?;
        Ok(())
    }
}
