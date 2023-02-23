pub mod kv;
mod table;

use std::fmt::Display;
use std::ops::{Bound, RangeBounds};

use crate::error::Result;
use crate::common::{KeyType, ValueType};

/// A key/value store.
pub trait Store: Display + Send + Sync {
    /// Sets a value for a key, replacing the existing value if any.
    fn set_or_insert(&mut self, key: &[u8], value: Vec<u8>) -> Result<()>;

    /// Gets a value for a key, if it exists.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Deletes a key, doing nothing if it does not exist.
    fn delete(&mut self, key: &[u8]) -> Result<()>;

    /// Iterates over an ordered range of key/value pairs.
    fn scan(&self, range: Range) -> Scan;

    /// Flushes any buffered data to the underlying storage medium.
    fn flush(&mut self) -> Result<()>;
}

/// A scan range wrapper.
pub struct Range {
    start: Bound<KeyType>,
    end: Bound<KeyType>,
}

impl Range {
    /// std::ops::Range does not support inclusive range bounds.
    pub fn from<R: RangeBounds<KeyType>>(range: R) -> Self {
        Self {
            start: match range.start_bound() {
                Bound::Included(v) => Bound::Included(v.to_vec()),
                Bound::Excluded(v) => Bound::Excluded(v.to_vec()),
                Bound::Unbounded => Bound::Unbounded,
            },
            end: match range.end_bound() {
                Bound::Included(v) => Bound::Included(v.to_vec()),
                Bound::Excluded(v) => Bound::Excluded(v.to_vec()),
                Bound::Unbounded => Bound::Unbounded,
            },
        }
    }
}

impl RangeBounds<Vec<u8>> for Range {
    fn start_bound(&self) -> Bound<&Vec<u8>> {
        match &self.start {
            Bound::Included(v) => Bound::Included(v),
            Bound::Excluded(v) => Bound::Excluded(v),
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    fn end_bound(&self) -> Bound<&Vec<u8>> {
        match &self.end {
            Bound::Included(v) => Bound::Included(v),
            Bound::Excluded(v) => Bound::Excluded(v),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}

/// Iterator over a key/value range.
pub type Scan = Box<dyn DoubleEndedIterator<Item = Result<(KeyType, ValueType)>> + Send>;

#[cfg(test)]
trait TestSuite<S: Store> {
    fn setup() -> Result<S>;

    fn test() -> Result<()> {
        Self::test_delete()?;
        Self::test_get()?;
        Self::test_scan()?;
        Self::test_set()?;
        Self::test_random()?;
        Ok(())
    }

    fn test_get() -> Result<()> {
        let mut s = Self::setup()?;
        s.set_or_insert(b"a", vec![0x01])?;
        assert_eq!(Some(vec![0x01]), s.get(b"a")?);
        assert_eq!(None, s.get(b"b")?);
        Ok(())
    }

    fn test_delete() -> Result<()> {
        let mut s = Self::setup()?;
        s.set_or_insert(b"a", vec![0x01])?;
        assert_eq!(Some(vec![0x01]), s.get(b"a")?);
        s.delete(b"a")?;
        assert_eq!(None, s.get(b"a")?);
        s.delete(b"b")?;
        Ok(())
    }

    fn test_random() -> Result<()> {
        use rand::Rng;
        let mut s = Self::setup()?;
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(397_427_893);

        // Create a bunch of random items and insert them
        let mut items: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for i in 0..1000_u64 {
            items.push((rng.gen::<[u8; 32]>().to_vec(), i.to_be_bytes().to_vec()))
        }
        for (key, value) in items.iter() {
            s.set_or_insert(key, value.clone())?;
        }

        // Fetch the random items, both via get() and scan()
        for (key, value) in items.iter() {
            assert_eq!(s.get(key)?, Some(value.clone()))
        }
        let mut expect = items.clone();
        expect.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(expect, s.scan(Range::from(..)).collect::<Result<Vec<_>>>()?);
        expect.reverse();
        assert_eq!(expect, s.scan(Range::from(..)).rev().collect::<Result<Vec<_>>>()?);

        // Remove the items
        for (key, _) in items {
            s.delete(&key)?;
            assert_eq!(None, s.get(&key)?);
        }
        assert!(s.scan(Range::from(..)).collect::<Result<Vec<_>>>()?.is_empty());

        Ok(())
    }

    fn test_scan() -> Result<()> {
        let mut s = Self::setup()?;
        s.set_or_insert(b"a", vec![0x01])?;
        s.set_or_insert(b"b", vec![0x02])?;
        s.set_or_insert(b"ba", vec![0x02, 0x01])?;
        s.set_or_insert(b"bb", vec![0x02, 0x02])?;
        s.set_or_insert(b"c", vec![0x03])?;

        // Forward/backward ranges
        assert_eq!(
            vec![
                (b"b".to_vec(), vec![0x02]),
                (b"ba".to_vec(), vec![0x02, 0x01]),
                (b"bb".to_vec(), vec![0x02, 0x02]),
            ],
            s.scan(Range::from(b"b".to_vec()..b"bz".to_vec())).collect::<Result<Vec<_>>>()?
        );
        assert_eq!(
            vec![
                (b"bb".to_vec(), vec![0x02, 0x02]),
                (b"ba".to_vec(), vec![0x02, 0x01]),
                (b"b".to_vec(), vec![0x02]),
            ],
            s.scan(Range::from(b"b".to_vec()..b"bz".to_vec())).rev().collect::<Result<Vec<_>>>()?
        );

        // Inclusive/exclusive ranges
        assert_eq!(
            vec![(b"b".to_vec(), vec![0x02]), (b"ba".to_vec(), vec![0x02, 0x01]),],
            s.scan(Range::from(b"b".to_vec()..b"bb".to_vec())).collect::<Result<Vec<_>>>()?
        );
        assert_eq!(
            vec![
                (b"b".to_vec(), vec![0x02]),
                (b"ba".to_vec(), vec![0x02, 0x01]),
                (b"bb".to_vec(), vec![0x02, 0x02]),
            ],
            s.scan(Range::from(b"b".to_vec()..=b"bb".to_vec())).collect::<Result<Vec<_>>>()?
        );

        // Open ranges
        assert_eq!(
            vec![(b"bb".to_vec(), vec![0x02, 0x02]), (b"c".to_vec(), vec![0x03]),],
            s.scan(Range::from(b"bb".to_vec()..)).collect::<Result<Vec<_>>>()?
        );
        assert_eq!(
            vec![(b"a".to_vec(), vec![0x01]), (b"b".to_vec(), vec![0x02]),],
            s.scan(Range::from(..=b"b".to_vec())).collect::<Result<Vec<_>>>()?
        );

        // Full range
        assert_eq!(
            vec![
                (b"a".to_vec(), vec![0x01]),
                (b"b".to_vec(), vec![0x02]),
                (b"ba".to_vec(), vec![0x02, 0x01]),
                (b"bb".to_vec(), vec![0x02, 0x02]),
                (b"c".to_vec(), vec![0x03]),
            ],
            s.scan(Range::from(..)).collect::<Result<Vec<_>>>()?
        );
        Ok(())
    }

    fn test_set() -> Result<()> {
        let mut s = Self::setup()?;
        s.set_or_insert(b"a", vec![0x01])?;
        assert_eq!(Some(vec![0x01]), s.get(b"a")?);
        s.set_or_insert(b"a", vec![0x02])?;
        assert_eq!(Some(vec![0x02]), s.get(b"a")?);
        Ok(())
    }
}
