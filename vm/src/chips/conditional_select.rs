// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::NUM_OF_ADVICE_COLUMNS;
use crate::chips::utilities::Expr;
use crate::value::Value;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct ConditionalSelectConfig {
    advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    s_cs: Selector,
}

pub struct ConditionalSelectChip<F: FieldExt> {
    config: ConditionalSelectConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for ConditionalSelectChip<F> {
    type Config = ConditionalSelectConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> ConditionalSelectChip<F> {
    pub fn construct(
        config: <Self as Chip<F>>::Config,
        _loaded: <Self as Chip<F>>::Loaded,
    ) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    ) -> <Self as Chip<F>>::Config {
        for column in &advices {
            meta.enable_equality(*column);
        }
        let s_cs = meta.selector();

        meta.create_gate("conditional_select", |meta| {
            let lhs = meta.query_advice(advices[0], Rotation::cur());
            let rhs = meta.query_advice(advices[1], Rotation::cur());
            let out = meta.query_advice(advices[2], Rotation::cur());
            let cond = meta.query_advice(advices[3], Rotation::cur());
            let s_cs = meta.query_selector(s_cs);

            vec![
                // cond is 0 or 1
                s_cs.clone() * (cond.clone() * (1.expr() - cond.clone())),
                // lhs * cond + rhs * (1 - cond) = out
                s_cs * ((lhs - rhs.clone()) * cond + rhs - out),
            ]
        });

        ConditionalSelectConfig { advices, s_cs }
    }

    pub fn conditional_select(
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
        cond: Option<F>,
    ) -> Result<Value<F>, Error> {
        let config = self.config();

        let mut c = None;
        layouter.assign_region(
            || "conditional_select",
            |mut region: Region<'_, F>| {
                config.s_cs.enable(&mut region, 0)?;

                let lhs = region.assign_advice(
                    || "lhs",
                    config.advices[0],
                    0,
                    || a.value().ok_or(Error::Synthesis),
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    config.advices[1],
                    0,
                    || b.value().ok_or(Error::Synthesis),
                )?;
                region.constrain_equal(a.cell().unwrap(), lhs.cell())?;
                region.constrain_equal(b.cell().unwrap(), rhs.cell())?;

                let value = match (a.value(), b.value(), cond) {
                    (Some(a), Some(b), Some(cond)) => {
                        let v = if cond == F::one() { a } else { b };
                        Some(v)
                    }
                    _ => None,
                };

                let cell = region.assign_advice(
                    || "select result",
                    config.advices[2],
                    0,
                    || value.ok_or(Error::Synthesis),
                )?;

                region.assign_advice(
                    || "cond",
                    config.advices[3],
                    0,
                    || cond.ok_or(Error::Synthesis),
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
