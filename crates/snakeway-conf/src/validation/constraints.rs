//! Reusable constraints

use confval::prelude::range_constraint;

range_constraint!(pub(crate) PORT, i64, min: 1, max: 65535);
range_constraint!(pub(crate) RESPONSE_CODE, i64, min: 300, max: 399);
