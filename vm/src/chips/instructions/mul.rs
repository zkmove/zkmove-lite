// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::NUM_OF_ADVICE_COLUMNS;
use crate::value::Value;
use crate::{assign_cond, assign_operands};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct MulConfig<F: FieldExt> {
    s_mul: Selector,
    advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    _marker: PhantomData<F>,
}

pub struct MulChip<F: FieldExt> {
    config: MulConfig<F>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for MulChip<F> {
    type Config = MulConfig<F>;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> MulChip<F> {
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
        let s_mul = meta.selector();
        meta.create_gate("mul", |meta| {
            let lhs = meta.query_advice(advices[0], Rotation::cur());
            let rhs = meta.query_advice(advices[1], Rotation::cur());
            let out = meta.query_advice(advices[2], Rotation::cur());
            let cond = meta.query_advice(advices[3], Rotation::cur());
            let s_mul = meta.query_selector(s_mul) * cond;

            vec![s_mul * (lhs * rhs - out)]
        });

        MulConfig {
            s_mul,
            advices,
            _marker: PhantomData,
        }
    }

    pub(crate) fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        let config = self.config();

        let mut c = None;
        layouter.assign_region(
            || "mul",
            |mut region: Region<'_, F>| {
                config.s_mul.enable(&mut region, 0)?;

                assign_operands!(a, b, region, config);
                assign_cond!(cond, region, config);

                let value = a.value().and_then(|a| b.value().map(|b| a * b));
                let cell = region.assign_advice(
                    || "lhs * rhs",
                    config.advices[2],
                    0,
                    || value.ok_or(Error::Synthesis),
                )?;
                c = Some(
                    Value::new_variable(value, Some(cell.cell()), a.ty())
                        .map_err(|_| Error::Synthesis)?,
                );
                Ok(())
            },
        )?;

        Ok(c.unwrap())
    }
}
