// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

use crate::chips::evaluation_chip::NUM_OF_ADVICE_COLUMNS;
use crate::chips::utilities::Expr;
use crate::value::Value;
use crate::{assign_cond, assign_delta_invert, assign_operands};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use movelang::value::MoveValueType;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct NeqConfig<F: FieldExt> {
    s_neq: Selector,
    advices: [Column<Advice>; NUM_OF_ADVICE_COLUMNS],
    _marker: PhantomData<F>,
}

pub struct NeqChip<F: FieldExt> {
    config: NeqConfig<F>,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for NeqChip<F> {
    type Config = NeqConfig<F>;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> NeqChip<F> {
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
        let s_neq = meta.selector();
        meta.create_gate("neq", |meta| {
            let lhs = meta.query_advice(advices[0], Rotation::cur());
            let rhs = meta.query_advice(advices[1], Rotation::cur());
            let out = meta.query_advice(advices[2], Rotation::cur());
            let cond = meta.query_advice(advices[3], Rotation::cur());
            let delta_invert = meta.query_advice(advices[0], Rotation::next());
            let s_neq = meta.query_selector(s_neq) * cond;

            vec![
                // out is 0 or 1
                s_neq.clone() * (out.clone() * (1.expr() - out.clone())),
                // if a != b then (a - b) * inverse(a - b) == out
                // if a == b then (a - b) * 1 == out
                s_neq.clone() * ((lhs.clone() - rhs.clone()) * delta_invert.clone() - out),
                // constrain delta_invert: (a - b) * inverse(a - b) must be 1 or 0
                s_neq * (lhs.clone() - rhs.clone()) * ((lhs - rhs) * delta_invert - 1.expr()),
            ]
        });

        NeqConfig {
            s_neq,
            advices,
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
            || "neq",
            |mut region: Region<'_, F>| {
                config.s_neq.enable(&mut region, 0)?;

                assign_operands!(a, b, region, config);
                assign_cond!(cond, region, config);
                assign_delta_invert!(a, b, region, config);

                let value = match (a.value(), b.value()) {
                    (Some(a), Some(b)) => {
                        let v = if a != b { F::one() } else { F::zero() };
                        Some(v)
                    }
                    _ => None,
                };

                let cell = region.assign_advice(
                    || "lhs != rhs",
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
}
