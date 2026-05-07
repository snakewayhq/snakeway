use crate::{Origin, ValidationReport};

pub trait ValidateSpec<O: Origin> {
    fn validate(&self, origin: &O, report: &mut ValidationReport<O>);
}
