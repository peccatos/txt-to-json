use crate::ast::{Contract, ContractFormula, ContractInvariant};
use crate::validator::ValidatedDocument;

pub fn build_contract(document: ValidatedDocument) -> Contract {
    Contract {
        meta: document.meta,
        formulas: document
            .formulas
            .into_iter()
            .map(|formula| ContractFormula {
                lhs: formula.lhs,
                rhs: formula.rhs.to_source(),
            })
            .collect(),
        invariants: document
            .invariants
            .into_iter()
            .map(|invariant| ContractInvariant {
                field: invariant.field,
                min: invariant.min,
                max: invariant.max,
            })
            .collect(),
        pipeline: document.pipeline.into_iter().map(|op| op.name).collect(),
    }
}
