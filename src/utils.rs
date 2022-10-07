use crate::error::Error;
use crate::CredxResult;
use group::{Group, GroupEncoding};
use indexmap::{IndexMap, IndexSet};
use serde::de::MapAccess;
use serde::ser::SerializeMap;
use serde::{
    de::{DeserializeOwned, Error as DError, SeqAccess, Unexpected, Visitor},
    ser::SerializeSeq,
    Deserializer, Serialize, Serializer,
};
use std::{
    fmt::{self, Formatter},
    hash::Hash,
    marker::PhantomData,
};
use yeti::knox::bls12_381_plus::{G1Affine, G1Projective, Scalar};

pub const TOP_BIT: u64 = i64::MIN as u64;

pub fn get_num_scalar(num: isize) -> Scalar {
    Scalar::from(zero_center(num))
}

pub fn zero_center(num: isize) -> u64 {
    num as u64 ^ TOP_BIT
}

pub fn serialize_point<P: Group + GroupEncoding + Serialize + DeserializeOwned, S: Serializer>(
    point: &P,
    s: S,
) -> Result<S::Ok, S::Error> {
    let bytes = point.to_bytes().as_ref().to_vec();
    s.serialize_bytes(bytes.as_slice())
}

pub fn deserialize_point<
    'de,
    P: Group + GroupEncoding + Serialize + DeserializeOwned,
    D: Deserializer<'de>,
>(
    d: D,
) -> Result<P, D::Error> {
    struct PointVisitor<PP: Group + GroupEncoding + Serialize + DeserializeOwned> {
        _marker: PhantomData<PP>,
    }

    impl<'de, PP: Group + GroupEncoding + Serialize + DeserializeOwned> Visitor<'de>
        for PointVisitor<PP>
    {
        type Value = PP;

        fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
            write!(formatter, "a byte sequence")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: DError,
        {
            let mut repr = PP::Repr::default();
            if repr.as_ref().len() != v.len() {
                return Err(DError::invalid_type(Unexpected::Bytes(v), &self));
            }
            repr.as_mut().copy_from_slice(v);
            let point = PP::from_bytes(&repr);
            if point.is_none().unwrap_u8() == 1u8 {
                return Err(DError::invalid_type(Unexpected::Bytes(v), &self));
            }
            Ok(point.unwrap())
        }
    }

    d.deserialize_bytes(PointVisitor::<P> {
        _marker: PhantomData,
    })
}

pub fn serialize_indexset<T: Serialize, S: Serializer>(
    set: &IndexSet<T>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut i = s.serialize_seq(Some(set.len()))?;
    for e in set {
        i.serialize_element(e)?;
    }
    i.end()
}

pub fn deserialize_indexset<'de, T: Eq + Hash + DeserializeOwned, D: Deserializer<'de>>(
    d: D,
) -> Result<IndexSet<T>, D::Error> {
    struct IndexSetVisitor<TT: Eq + DeserializeOwned> {
        _marker: PhantomData<TT>,
    }

    impl<'de, TT: Eq + Hash + DeserializeOwned> Visitor<'de> for IndexSetVisitor<TT> {
        type Value = IndexSet<TT>;

        fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
            write!(formatter, "a sequence")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut set = IndexSet::new();
            while let Some(e) = seq.next_element()? {
                set.insert(e);
            }
            Ok(set)
        }
    }

    d.deserialize_seq(IndexSetVisitor::<T> {
        _marker: PhantomData,
    })
}

pub fn serialize_indexmap<K: Serialize, V: Serialize, S: Serializer>(
    map: &IndexMap<K, V>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut i = s.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        i.serialize_entry(k, v)?;
    }
    i.end()
}

pub fn deserialize_indexmap<
    'de,
    K: Eq + Hash + DeserializeOwned,
    V: DeserializeOwned,
    D: Deserializer<'de>,
>(
    d: D,
) -> Result<IndexMap<K, V>, D::Error> {
    struct IndexMapVisitor<KK: Eq + Hash + DeserializeOwned, VV: DeserializeOwned> {
        _key_marker: PhantomData<KK>,
        _value_marker: PhantomData<VV>,
    }

    impl<'de, KK: Eq + Hash + DeserializeOwned, VV: DeserializeOwned> Visitor<'de>
        for IndexMapVisitor<KK, VV>
    {
        type Value = IndexMap<KK, VV>;

        fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
            write!(formatter, "a map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut m = IndexMap::new();
            while let Some((k, v)) = map.next_entry()? {
                m.insert(k, v);
            }
            Ok(m)
        }
    }

    d.deserialize_map(IndexMapVisitor::<K, V> {
        _key_marker: PhantomData,
        _value_marker: PhantomData,
    })
}

pub fn scalar_from_hex_str(sc: &str, e: Error) -> CredxResult<Scalar> {
    let bytes = hex::decode(sc).map_err(|_| e)?;
    let buf = <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| e)?;
    let sr = Scalar::from_bytes(&buf);
    if sr.is_some().unwrap_u8() == 1 {
        Ok(sr.unwrap())
    } else {
        Err(Error::DeserializationError)
    }
}

pub fn g1_from_hex_str(g1: &str, e: Error) -> CredxResult<G1Projective> {
    let bytes = hex::decode(g1).map_err(|_| e)?;

    let buf = <[u8; 48]>::try_from(bytes.as_slice()).map_err(|_| e)?;
    let pt = G1Affine::from_compressed(&buf).map(G1Projective::from);
    if pt.is_some().unwrap_u8() == 1 {
        Ok(pt.unwrap())
    } else {
        Err(Error::DeserializationError)
    }
}
