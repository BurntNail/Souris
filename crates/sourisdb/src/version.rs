use crate::utilities::cursor::Cursor;
use core::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug)]
pub enum Version {
    V0_1_0,
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum VersionSerError {
    Invalid,
    NotEnoughBytes,
}

impl Display for VersionSerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            VersionSerError::NotEnoughBytes => write!(f, "Not enough bytes provided"),
            VersionSerError::Invalid => write!(f, "Invalid bytes provided"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VersionSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None //not using the pre-made function to remind me not to forget this later
    }
}

impl Version {
    #[must_use]
    pub fn to_bytes(self) -> &'static [u8] {
        match self {
            Self::V0_1_0 => b"V0_1_0",
        }
    }

    pub fn from_bytes(cursor: &mut Cursor<u8>) -> Result<Self, VersionSerError> {
        match cursor.read(6).ok_or(VersionSerError::NotEnoughBytes)? {
            b"V0_1_0" => Ok(Self::V0_1_0),
            _ => Err(VersionSerError::Invalid),
        }
    }
}

//TODO: actually work this out
/*#[cfg(test)]
mod strat {
    use proptest::arbitrary::Arbitrary;
    use proptest::prelude::Strategy;
    use proptest::strategy::{NewTree, ValueTree};
    use proptest::test_runner::TestRunner;
    use crate::version::Version;

    struct VersionValueTree {
        current: Version,
    }

    impl ValueTree for VersionValueTree {
        type Value = Version;

        fn current(&self) -> Self::Value {
            self.current
        }

        fn simplify(&mut self) -> bool {
            false
        }

        fn complicate(&mut self) -> bool {
            false
        }
    }

    #[derive(Debug)]
    struct VersionStrategy(u8);

    impl Strategy for VersionStrategy {
        type Tree = VersionValueTree;
        type Value = Version;

        fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
            Ok(VersionValueTree {
                current: Version::V0_1_0
            })
        }
    }

    impl Arbitrary for Version {
        type Parameters = u8;

        fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
            VersionStrategy(args)
        }

        type Strategy = VersionStrategy;
    }

}*/
