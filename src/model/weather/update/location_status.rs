use super::status::LocationUpdateStatus;
use crate::model::LocationZoneCode;
use multi_index_map::MultiIndexMap;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(MultiIndexMap, Debug, Clone, PartialEq, Eq, Hash)]
#[multi_index_derive(Debug, Clone)]
pub struct LocationStatus {
    #[multi_index(hashed_unique)]
    pub zone: LocationZoneCode,

    #[multi_index(hashed_non_unique)]
    pub status: LocationUpdateStatus,
}

impl LocationStatus {
    pub fn new(zone: LocationZoneCode) -> Self {
        Self { zone, status: LocationUpdateStatus::default() }
    }
}

impl PartialEq for MultiIndexLocationStatusMap {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        let lhs_iter = self.iter();
        let mut rhs_iter = other.iter();

        for lhs in lhs_iter {
            match rhs_iter.next() {
                None => return false,
                Some(rhs) if lhs.1 != rhs.1 => return false,
                Some(_) => {},
            }
        }

        true
    }
}

impl Eq for MultiIndexLocationStatusMap {}

impl Serialize for MultiIndexLocationStatusMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (_, ls) in self.iter() {
            map.serialize_entry(&ls.zone, &ls.status)?;
        }
        map.end()
    }
}

struct MultiIndexLocationStatusMapVisitor;

impl<'de> Visitor<'de> for MultiIndexLocationStatusMapVisitor {
    type Value = MultiIndexLocationStatusMap;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("multi-index location status map")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = MultiIndexLocationStatusMap::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((zone, status)) =
            access.next_entry::<LocationZoneCode, LocationUpdateStatus>()?
        {
            map.insert(LocationStatus { zone, status });
        }

        Ok(map)
    }
}

impl<'de> Deserialize<'de> for MultiIndexLocationStatusMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(MultiIndexLocationStatusMapVisitor)
    }
}
