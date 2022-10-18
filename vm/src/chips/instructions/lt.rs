// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::NUM_OF_ADVICE_COLUMNS;
use crate::chips::utilities::{ByteRepresentation, Expr, NUM_OF_BYTES_U128};
use crate::value::Value;
use crate::{assign_cond, assign_operands};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use logger::prelude::*;
use movelang::value::MoveValueType;
use std::convert::TryInto;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct LtConfig<F: FieldExt> {
    s_lt: Selector,
    advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    lt_diff_cells: ByteRepresentation<F, NUM_OF_BYTES_U128>,
    _marker: PhantomData<F>,
}

pub struct LtChip<F: FieldExt> {
    config: LtConfig<F>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for LtChip<F> {
    type Config = LtConfig<F>;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> LtChip<F> {
    pub(crate) fn construct(
        config: <Self as Chip<F>>::Config,
        _loaded: <Self as Chip<F>>::Loaded,
    ) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    ) -> <Self as Chip<F>>::Config {
        let s_lt = meta.selector();
        let mut lt_diff_cells = None;
        meta.create_gate("lt", |meta| {
            let lhs = meta.query_advice(advices[0], Rotation::cur());
            let rhs = meta.query_advice(advices[1], Rotation::cur());
            let out = meta.query_advice(advices[2], Rotation::cur());
            let cond = meta.query_advice(advices[3], Rotation::cur());
            let range = F::from(2).pow(&[(NUM_OF_BYTES_U128 * 8) as u64, 0, 0, 0]);
            let diff_cells =
                ByteRepresentation::construct(meta, advices, Rotation::next().0 as usize);
            lt_diff_cells = Some(diff_cells.clone());
            let s_lt = meta.query_selector(s_lt) * cond;

            vec![
                // out is 0 or 1
                s_lt.clone() * (out.clone() * (1.expr() - out.clone())),
                // let diff = if lhs >= rhs {lhs - rhs} else {lhs - rhs + range};
                // to constrain: lhs - rhs = diff - out * range
                // if lhs >= rhs, then diff = lhs - rhs, out must be 0.
                // if lhs < rhs, then diff = lhs - rhs + range, diff is in range 2^128, out can only be 1.
                //
                // diff is reconstructed from the lower 16 bytes of the original value,
                // it will always be in range 2^128.
                //
                s_lt * ((lhs - rhs) + out * range - diff_cells.expr()),
            ]
        });

        LtConfig {
            s_lt,
            advices,
            lt_diff_cells: lt_diff_cells.expect("lt_diff_cells is None."),
            _marker: PhantomData,
        }
    }

    pub(crate) fn assign(
        &self,
        layouter: &mut impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        let config = self.config();

        let mut c = None;
        layouter.assign_region(
            || "lt",
            |mut region: Region<'_, F>| {
                config.s_lt.enable(&mut region, 0)?;

                assign_operands!(a, b, region, config);
                assign_cond!(cond, region, config);
                self.assign_diff_bytes(&mut region, &config.lt_diff_cells, a.clone(), b.clone())?;

                let value = match (a.value(), b.value()) {
                    (Some(a), Some(b)) => {
                        let v = if a < b { F::one() } else { F::zero() };
                        Some(v)
                    }
                    _ => None,
                };

                let cell = region.assign_advice(
                    || "lhs < rhs",
                    config.advices[2],
                    0,
                    || value.ok_or(Error::Synthesis),
                )?;

                c = Some(
                    Value::new_variable(value, Some(cell.cell()), MoveValueType::Bool)
                        .map_err(|_| Error::Synthesis)?,
                );
                Ok(())
            },
        )?;

        Ok(c.unwrap())
    }

    /// if a >= b then diff = a - b; if a < b then diff = a - b + range;
    /// Move doesn't support u256, range 0~2^128 is workable for u8, u64, u128
    /// convert diff into the byte representation, assign them into specified cells.
    fn assign_diff_bytes(
        &self,
        region: &mut Region<'_, F>,
        cells: &ByteRepresentation<F, NUM_OF_BYTES_U128>,
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
