// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::NUM_OF_ADVICE_COLUMNS;
use crate::value::Value;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};
use movelang::value::MoveValueType;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct NotConfig<F: FieldExt> {
    s_not: Selector,
    advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    _marker: PhantomData<F>,
}

pub struct NotChip<F: FieldExt> {
    config: NotConfig<F>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for NotChip<F> {
    type Config = NotConfig<F>;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> NotChip<F> {
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
        let s_not = meta.selector();
        meta.create_gate("not", |meta| {
            let x = meta.query_advice(advices[0], Rotation::cur());
            let out = meta.query_advice(advices[1], Rotation::cur());
            let cond = meta.query_advice(advices[2], Rotation::cur());
            let s_not = meta.query_selector(s_not) * cond;
            let one = Expression::Constant(F::one());

            vec![
                // 1 - x = out
                s_not * (one - x - out),
            ]
        });

        NotConfig {
            s_not,
            advices,
            _marker: PhantomData,
        }
    }

    pub(crate) fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        let config = self.config();

        let mut b = None;
        layouter.assign_region(
            || "not",
            |mut region: Region<'_, F>| {
                config.s_not.enable(&mut region, 0)?;

                // assign operand
                let x = region.assign_advice(
                    || "x",
                    config.advices[0],
                    0,
                    || a.value().ok_or(Error::Synthesis),
                )?;
                region.constrain_equal(a.cell().unwrap(), x.cell())?;

                // assign cond
                region.assign_advice(
                    || "cond",
                    config.advices[2],
                    0,
                    || cond.ok_or(Error::Synthesis),
                )?;

                let value = match a.value() {
                    Some(a) => {
                        let v = if a == F::zero() { F::one() } else { F::zero() };
                        Some(v)
                    }
                    _ => None,
                };

                let cell = region.assign_advice(
                    || "not a",
                    config.advices[1],
                    0,
                    || value.ok_or(Error::Synthesis),
                )?;

                b = Some(
                    Value::new_variable(value, Some(cell.cell()), MoveValueType::Bool)
                        .map_err(|_| Error::Synthesis)?,
                );
                Ok(())
            },
        )?;

        Ok(b.unwrap())
    }
}
