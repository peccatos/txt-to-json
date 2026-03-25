use crate::ast::Contract;
use crate::validator::ValidatedDocument;

pub fn build_contract(document: ValidatedDocument) -> Contract {
    Contract {
        meta: document.meta,
        formulas: document.formulas,
        invariants: document.invariants,
        pipeline: document.pipeline,
    }
}
