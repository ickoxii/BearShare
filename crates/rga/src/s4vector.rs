use serde::{Deserialize, Serialize};

/// S4Vector: A quadruple vector for globally unique operation identifiers
/// Derived from vector clocks with fixed size for efficiency
///
/// Fields (from Section 5.1):
/// - ssn: session number (increments on membership changes)
/// - sid: site ID (unique to each site)
/// - sum: sum of all vector clock components
/// - seq: sequence number for tombstone purging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct S4Vector {
    pub ssn: u32,
    pub sid: u32,
    pub sum: u32,
    pub seq: u32,
}

impl S4Vector {
    pub fn new(ssn: u32, sid: u32, sum: u32, seq: u32) -> Self {
        S4Vector { ssn, sid, sum, seq }
    }

    /// S4Vector ordering as defined in Definition 9
    /// Implements Precedence Transitivity (PT) through total ordering
    pub fn precedes(&self, other: &S4Vector) -> bool {
        // Condition 1: Different sessions - order by session number
        if self.ssn != other.ssn {
            return self.ssn < other.ssn;
        }

        // Condition 2: Same session, different sums - order by sum
        if self.sum != other.sum {
            return self.sum < other.sum;
        }

        // Condition 3: Same session and sum - order by site ID
        self.sid < other.sid
    }
}

impl PartialOrd for S4Vector {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for S4Vector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.precedes(other) {
            std::cmp::Ordering::Less
        } else if other.precedes(self) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::S4Vector;

    #[test]
    fn test_s4vector_ordering() {
        let v1 = S4Vector::new(1, 0, 1, 1);
        let v2 = S4Vector::new(1, 1, 1, 1);
        let v3 = S4Vector::new(1, 0, 2, 2);

        assert!(v1.precedes(&v2));
        assert!(v1.precedes(&v3));
        assert!(v2.precedes(&v3));

        // Transitivity check
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }
}
