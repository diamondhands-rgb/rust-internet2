// LNP/BP Core Library implementing LNPBP specifications & standards
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::any::Any;
use std::collections::BTreeMap;
use std::io;
use std::marker::PhantomData;
use std::sync::Arc;

use amplify::Wrapper;
use lightning_encoding::LightningDecode;
use strict_encoding::{self, StrictDecode};

use super::{EncodingType, Error, EvenOdd, Payload, TypeId, TypedEnum};

pub trait Unmarshall {
    type Data;
    type Error: std::error::Error;
    fn unmarshall(
        &self,
        reader: impl io::Read,
    ) -> Result<Self::Data, Self::Error>;
}

pub type UnmarshallFn<E> =
    fn(reader: &mut dyn io::Read) -> Result<Arc<dyn Any>, E>;

pub trait CreateUnmarshaller: Sized + TypedEnum {
    fn create_unmarshaller() -> Unmarshaller<Self>;
}

#[derive(Clone)]
pub struct Unmarshaller<T>
where
    T: TypedEnum,
{
    known_types: BTreeMap<TypeId, UnmarshallFn<Error>>,
    encoding: EncodingType,
    _phantom: PhantomData<T>,
}

impl<T> Unmarshall for Unmarshaller<T>
where
    T: TypedEnum,
{
    type Data = Arc<T>;
    type Error = Error;

    fn unmarshall(
        &self,
        mut reader: impl io::Read,
    ) -> Result<Self::Data, Self::Error> {
        let type_id = match self.encoding {
            EncodingType::Lightning => TypeId::lightning_decode(&mut reader)?,
            EncodingType::Strict => TypeId::strict_decode(&mut reader)?,
        };
        match self.known_types.get(&type_id) {
            None if type_id.is_even() => Err(Error::MessageEvenType(type_id)),
            None => {
                let mut payload = Vec::new();
                reader.read_to_end(&mut payload)?;
                Ok(Arc::new(T::try_from_type(type_id, &Payload {
                    type_id,
                    payload,
                })?))
            }
            Some(parser) => parser(&mut reader).and_then(|data| {
                Ok(Arc::new(T::try_from_type(type_id, &*data)?))
            }),
        }
    }
}

impl<T> Unmarshaller<T>
where
    T: TypedEnum,
{
    pub fn new(
        known_types: BTreeMap<u16, UnmarshallFn<Error>>,
        encoding: EncodingType,
    ) -> Self {
        Self {
            known_types: known_types
                .into_iter()
                .map(|(t, f)| (TypeId::from_inner(t), f))
                .collect(),
            encoding,
            _phantom: PhantomData,
        }
    }
}
