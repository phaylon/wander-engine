use fnv::{FnvHashMap};

use crate::behavior::Value;

use super::{World};
use super::entities::{Entity};


#[derive(Debug, Clone, Default)]
pub(super) struct WorldAgents {
    agent_locations: FnvHashMap<Entity, Entity>,
    agent_position: FnvHashMap<Entity, Value>,
}

impl World {
    pub fn create_agent(&mut self, location: Entity) -> Entity {
        assert!(self.is_area(location));
        let agent = self.spawn();
        self.agents.agent_locations.insert(agent, location);
        agent
    }

    pub fn agents(&self) -> impl Iterator<Item = Entity> + '_ {
        self.agents.agent_locations.keys().copied()
    }

    pub fn is_agent(&self, entity: Entity) -> bool {
        self.agents.agent_locations.contains_key(&entity)
    }

    pub fn set_agent_location(&mut self, agent: Entity, location: Entity) {
        let location_storage = self.agents.agent_locations.get_mut(&agent).expect("valid agent");
        *location_storage = location;
    }

    pub fn agent_location(&self, entity: Entity) -> Option<Entity> {
        self.agents.agent_locations.get(&entity).copied()
    }

    pub fn set_agent_position(&mut self, agent: Entity, position: Value) {
        assert!(self.is_agent(agent));
        self.agents.agent_position.insert(agent, position);
    }

    pub fn clear_agent_position(&mut self, agent: Entity) {
        assert!(self.is_agent(agent));
        self.agents.agent_position.remove(&agent);
    }

    pub fn agent_position(&self, agent: Entity) -> Option<&Value> {
        self.agents.agent_position.get(&agent)
    }
}