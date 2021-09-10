use crate::value::Value;
use bellman::pairing::Engine;
use bellman::{ConstraintSystem, SynthesisError};
use error::{RuntimeError, StatusCode, VmResult};
use ff::Field;

pub fn add<E, CS>(cs: &mut CS, left: Value<E>, right: Value<E>) -> VmResult<Value<E>>
where
    E: Engine,
    CS: ConstraintSystem<E>,
{
    let ty = left.ty();
    let value = match (left.value(), right.value()) {
        (Some(a), Some(b)) => {
            let mut add_result = a;
            add_result.add_assign(&b);
            Some(add_result)
        }
        _ => None,
    };

    let variable = cs
        .alloc(
            || "variable",
            || value.ok_or(SynthesisError::AssignmentMissing),
        )
        .map_err(|e| RuntimeError::new(StatusCode::SynthesisError(e)))?;

    cs.enforce(
        || "constraint",
        |lc| lc + &left.lc::<CS>() + &right.lc::<CS>(),
        |lc| lc + CS::one(),
        |lc| lc + variable,
    );

    Value::new_variable(value, variable, ty)
}
