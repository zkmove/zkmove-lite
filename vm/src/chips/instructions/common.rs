// Copyright (c) zkMove Authors
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! assign_operands {
    ($a:expr, $b:expr, $region:expr, $config:expr) => {{
        let lhs = $region.assign_advice(
            || "lhs",
            $config.advices[0],
            0,
            || $a.value().ok_or(Error::Synthesis),
        )?;
        let rhs = $region.assign_advice(
            || "rhs",
            $config.advices[1],
            0,
            || $b.value().ok_or(Error::Synthesis),
        )?;
        $region.constrain_equal($a.cell().unwrap(), lhs.cell())?;
        $region.constrain_equal($b.cell().unwrap(), rhs.cell())?;
    }};
}

#[macro_export]
macro_rules! assign_cond {
    ($cond:expr, $region:expr, $config:expr) => {{
        $region.assign_advice(
            || "cond",
            $config.advices[3],
            0,
            || $cond.ok_or(Error::Synthesis),
        )?;
    }};
}

#[macro_export]
macro_rules! div_rem {
    ($a:expr, $b:expr) => {{
        let l_move: Option<MoveValue> = $a.clone().into();
        let r_move: Option<MoveValue> = $b.clone().into();
        match (l_move, r_move) {
            (Some(l), Some(r)) => {
                let quo = move_div(l.clone(), r.clone()).map_err(|e| {
                    error!("move div failed: {:?}", e);
                    Error::Synthesis
                })?;
                let rem = move_rem(l, r).map_err(|e| {
                    error!("move rem failed: {:?}", e);
                    Error::Synthesis
                })?;
                (
                    Some(convert_to_field::<F>(quo)),
                    Some(convert_to_field::<F>(rem)),
                )
            }
            _ => (None, None),
        }
    }};
}

#[macro_export]
macro_rules! assign_delta_invert {
    ($a:expr, $b:expr, $region:expr, $config:expr) => {{
        $region.assign_advice(
            || "delta invert",
            $config.advices[0],
            1,
            || {
                let delta_invert = if $a.value() == $b.value() {
                    F::one()
                } else {
                    let delta = $a.value().unwrap() - $b.value().unwrap();
                    delta.invert().unwrap()
                };
                Ok(delta_invert)
            },
        )?;
    }};
}
