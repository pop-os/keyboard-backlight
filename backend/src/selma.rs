use serde::{Deserialize, Serialize};
use std::cmp;

use crate::Matrix;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum SelmaKind {
    Normal,
    Bouncing,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Selma {
    pub missing: Matrix,
    pub bouncing: Matrix,
    pub sticking: Matrix,
}

impl Selma {
    pub fn max_rows(&self) -> usize {
        cmp::max(
            self.missing.rows(),
            cmp::max(self.bouncing.rows(), self.sticking.rows()),
        )
    }

    pub fn max_cols(&self) -> usize {
        cmp::max(
            self.missing.cols(),
            cmp::max(self.bouncing.cols(), self.sticking.cols()),
        )
    }

    pub fn success(&self) -> bool {
        for matrix in &[&self.missing, &self.bouncing, &self.sticking] {
            for row in 0..matrix.rows() {
                for col in 0..matrix.cols() {
                    if matrix.get(row, col).unwrap_or(false) {
                        return false;
                    }
                }
            }
        }
        true
    }
}
