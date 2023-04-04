
use self::agents::WorldAgents;
use self::entities::WorldEntities;
use self::layout::WorldLayout;


pub mod entities;
pub mod layout;
pub mod agents;

#[derive(Debug, Clone, Default)]
pub struct World {
    entities: WorldEntities,
    layout: WorldLayout,
    agents: WorldAgents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct InvalidEntity;

pub type EntityResult<T = ()> = Result<T, InvalidEntity>;