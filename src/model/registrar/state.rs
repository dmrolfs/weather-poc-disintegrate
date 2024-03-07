use crate::model::registrar::protocol::RegistrarEvent;
use crate::model::LocationZoneCode;
use disintegrate::{StateMutate, StateQuery};
use smol_str::SmolStr;
use std::collections::HashSet;
use tagid::{Entity, IdGenerator, Label};

// #[cfg(test)]
// use coerce_cqrs_test::fixtures::aggregate::{Summarizable, Summarize};

// pub type RegistrarAggregate = coerce::actor::LocalActorRef<Registrar>;

// pub type RegistrarId = Id<Registrar, SmolStr>;

// static SINGLETON_ID: OnceCell<RegistrarId> = OnceCell::new();

// #[inline]
// pub fn singleton_id() -> &'static RegistrarId {
//     SINGLETON_ID.get_or_init(Registrar::next_id)
// }

#[derive(Debug, Clone, Default, StateQuery, Label, Serialize, Deserialize)]
#[state_query(RegistrarEvent)]
pub struct Registrar {
    pub location_codes: HashSet<LocationZoneCode>,
    // services: RegistrarServicesRef,
}

impl StateMutate for Registrar {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            RegistrarEvent::ForecastZoneAdded { zone } => {
                self.location_codes.insert(zone);
            },
            RegistrarEvent::ForecastZoneRemoved { zone } => {
                self.location_codes.remove(&zone);
            },
            RegistrarEvent::AllForecastZonesRemoved => {
                self.location_codes.clear();
            },
        }
    }
}

pub struct SingletonIdGenerator;

const REGISTRAR_SINGLETON_ID: &str = "<singleton>";

impl IdGenerator for SingletonIdGenerator {
    type IdType = SmolStr;

    fn next_id_rep() -> Self::IdType {
        SmolStr::new(REGISTRAR_SINGLETON_ID)
    }
}

impl Entity for Registrar {
    type IdGen = SingletonIdGenerator;
}
