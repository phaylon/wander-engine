use fnv::{FnvHashMap, FnvHashSet};
use smol_str::SmolStr;

use crate::behavior::Value;
use crate::util::{UnwrapOrEmptyIter};

use super::{World, InvalidEntity, EntityResult};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity(u32);

pub type EntitySet = FnvHashSet<Entity>;

#[derive(Debug, Clone)]
struct EntityMeta {
    identifier: Option<SmolStr>,
    global_attributes: FnvHashMap<Value, Value>,
    global_tags: FnvHashSet<Value>,
    agent_attributes: FnvHashMap<Entity, FnvHashMap<Value, Value>>,
    agent_tags: FnvHashMap<Entity, FnvHashSet<Value>>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct WorldEntities {
    next_entity_id: u32,
    meta: FnvHashMap<Entity, EntityMeta>,
}

/// General entity management.
impl World {
    pub(super) fn spawn(&mut self) -> Entity {
        let idx = self.entities.next_entity_id;
        self.entities.next_entity_id = idx.checked_add(1).expect("entity sequence exhausted");
        let entity = Entity(idx);
        self.entities.meta.insert(entity, EntityMeta {
            identifier: None,
            global_attributes: FnvHashMap::default(),
            global_tags: FnvHashSet::default(),
            agent_attributes: FnvHashMap::default(),
            agent_tags: FnvHashMap::default(),
        });
        entity
    }

    pub(super) fn despawn(&mut self, entity: Entity) {
        self.entities.meta.remove(&entity);
        for meta in self.entities.meta.values_mut() {
            meta.agent_attributes.remove(&entity);
        }
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.meta.contains_key(&entity)
    }

    fn meta(&self, entity: Entity) -> EntityResult<&EntityMeta> {
        self.entities.meta.get(&entity).ok_or(InvalidEntity)
    }

    fn meta_mut(&mut self, entity: Entity) -> EntityResult<&mut EntityMeta> {
        self.entities.meta.get_mut(&entity).ok_or(InvalidEntity)
    }
}

/// Entity identification.
impl World {
    pub fn set_identifier<T>(&mut self, entity: Entity, identifier: T)
    where
        T: Into<SmolStr>,
    {
        self.meta_mut(entity).expect("valid entity").identifier = Some(identifier.into());
    }

    pub fn identifier(&self, entity: Entity) -> Option<&SmolStr> {
        self.meta(entity).ok().and_then(|meta| meta.identifier.as_ref())
    }
}

/// Global entity attributes.
impl World {
    pub fn set_global_attribute_value(
        &mut self,
        entity: Entity,
        attr: Value,
        value: Value,
    ) -> EntityResult {
        self.meta_mut(entity)?.global_attributes.insert(attr, value);
        Ok(())
    }

    pub fn clear_global_attribute_value(
        &mut self,
        entity: Entity,
        attr: &Value,
    ) -> EntityResult<Option<Value>> {
        Ok(self.meta_mut(entity)?.global_attributes.remove(attr))
    }

    pub fn global_attribute_value(
        &self,
        entity: Entity,
        attr: &Value,
    ) -> EntityResult<Option<&Value>> {
        Ok(self.meta(entity)?.global_attributes.get(attr))
    }

    pub fn global_attributes(
        &self,
        entity: Entity,
    ) -> EntityResult<impl Iterator<Item = (&Value, &Value)> + '_> {
        Ok(self.meta(entity)?.global_attributes.iter())
    }
}

/// Global entity tags.
impl World {
    pub fn set_global_tag(&mut self, entity: Entity, tag: Value) -> EntityResult {
        self.meta_mut(entity)?.global_tags.insert(tag);
        Ok(())
    }

    pub fn clear_global_tag(&mut self, entity: Entity, tag: &Value) -> EntityResult {
        self.meta_mut(entity)?.global_tags.remove(tag);
        Ok(())
    }

    pub fn global_tags(&self, entity: Entity) -> EntityResult<impl Iterator<Item = &Value> + '_> {
        Ok(self.meta(entity)?.global_tags.iter())
    }

    pub fn contains_global_tag(&self, entity: Entity, tag: &Value) -> EntityResult<bool> {
        Ok(self.meta(entity)?.global_tags.contains(tag))
    }
}

/// Agent local entity attributes.
impl World {
    pub fn set_agent_attribute_value(
        &mut self,
        agent: Entity,
        entity: Entity,
        attr: Value,
        value: Value,
    ) -> EntityResult {
        self.meta_mut(entity)?
            .agent_attributes.entry(agent).or_default()
            .insert(attr, value);
        Ok(())
    }

    pub fn clear_agent_attribute_value(
        &mut self,
        agent: Entity,
        entity: Entity,
        attr: &Value,
    ) -> EntityResult<Option<Value>> {
        self.meta_mut(entity).map(|meta| {
            meta.agent_attributes.get_mut(&agent)?.remove(attr)
        })
    }

    pub fn agent_attribute_value(
        &self,
        agent: Entity,
        entity: Entity,
        attr: &Value,
    ) -> EntityResult<Option<&Value>> {
        self.meta(entity).map(|meta| {
            meta.agent_attributes.get(&agent)?.get(attr)
        })
    }

    pub fn agent_attributes(
        &self,
        agent: Entity,
        entity: Entity,
    ) -> EntityResult<impl Iterator<Item = (&Value, &Value)> + '_> {
        Ok(self.meta(entity)?.agent_attributes.get(&agent).unwrap_or_empty_iter())
    }
}

/// Agent local entity tags.
impl World {
    pub fn set_agent_tag(
        &mut self,
        agent: Entity,
        entity: Entity,
        tag: Value,
    ) -> EntityResult {
        self.meta_mut(entity)?.agent_tags.entry(agent).or_default().insert(tag);
        Ok(())
    }

    pub fn clear_agent_tag(
        &mut self,
        agent: Entity,
        entity: Entity,
        tag: &Value,
    ) -> EntityResult {
        self.meta_mut(entity)?.agent_tags.get_mut(&agent).map(|tags| tags.remove(tag));
        Ok(())
    }

    pub fn agent_tags(
        &self,
        agent: Entity,
        entity: Entity,
    ) -> EntityResult<impl Iterator<Item = &Value> + '_> {
        Ok(self.meta(entity)?.agent_tags.get(&agent).unwrap_or_empty_iter())
    }

    pub fn has_agent_tag(
        &self,
        agent: Entity,
        entity: Entity,
        tag: &Value,
    ) -> EntityResult<bool> {
        Ok(self.meta(entity)?.agent_tags.get(&agent).map_or(false, |tags| tags.contains(tag)))
    }
}