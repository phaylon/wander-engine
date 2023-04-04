use std::collections::VecDeque;
use std::sync::Arc;

use coupled::Pair;
use fnv::FnvHashMap;
use smallvec::SmallVec;

use crate::behavior::Value;

use super::World;
use super::entities::{EntitySet, Entity};


type LocalBuffer<T> = SmallVec<[T; 64]>;

#[derive(Debug, Clone, Default)]
pub(super) struct WorldLayout {
    spaces: EntitySet,
    object_parents: FnvHashMap<Entity, Entity>,
    kinds: FnvHashMap<Entity, Value>,
    portals: EntitySet,
    portal_objects: FnvHashMap<Entity, PortalTarget>,
    paths: FnvHashMap<(Entity, Entity), Vec<Value>>,
    space_distances: FnvHashMap<Pair<Entity>, usize>,
}

#[derive(Debug, Clone)]
struct PortalTarget {
    portal: Entity,
    target_object: Entity,
}

impl World {
    pub fn layout_kind(&self, entity: Entity) -> Option<&Value> {
        self.layout.kinds.get(&entity)
    }

    pub fn create_space(&mut self, kind: Value) -> Entity {
        let entity = self.spawn();
        self.layout.spaces.insert(entity);
        self.layout.kinds.insert(entity, kind);
        self.recalculate();
        entity
    }

    pub fn is_space(&self, entity: Entity) -> bool {
        self.layout.spaces.contains(&entity)
    }

    pub fn spaces(&self) -> impl Iterator<Item = Entity> + '_ {
        self.layout.spaces.iter().copied()
    }

    pub fn spaces_by_distance(&self, source: Entity) -> impl Iterator<Item = Entity> + '_ {
        let mut spaces = self.spaces().filter_map(|space| {
            let dist = self.layout.space_distances.get(&Pair::new(source, space)).copied()?;
            Some((space, dist))
        }).collect::<LocalBuffer<_>>();
        spaces.sort_by_key(|(_, dist)| *dist);
        spaces.into_iter().map(|(space, _)| space)
    }

    pub fn create_object(&mut self, kind: Value, parent: Entity) -> Entity {
        assert!(self.is_space(parent) || self.is_object(parent));
        let entity = self.spawn();
        self.layout.object_parents.insert(entity, parent);
        self.layout.kinds.insert(entity, kind);
        self.recalculate();
        entity
    }

    pub fn is_object(&self, entity: Entity) -> bool {
        self.layout.object_parents.contains_key(&entity)
    }

    pub fn is_area(&self, entity: Entity) -> bool {
        self.is_object(entity)
        && self.object_parent(entity).map_or(false, |parent| self.is_space(parent))
    }

    pub fn areas(&self) -> impl Iterator<Item = Entity> + '_ {
        self.spaces().flat_map(|space| self.child_objects(space))
    }

    pub fn child_objects(&self, parent: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.layout.object_parents.iter().filter_map(move |(object, object_parent)| {
            if parent == *object_parent {
                Some(*object)
            } else {
                None
            }
        })
    }

    pub fn object_parent(&self, object: Entity) -> Option<Entity> {
        self.layout.object_parents.get(&object).copied()
    }

    pub fn object_space(&self, object: Entity) -> Option<Entity> {
        let mut entity = object;
        while !self.is_space(entity) {
            entity = self.object_parent(entity)?;
        }
        Some(entity)
    }

    pub fn object_area(&self, object: Entity) -> Option<Entity> {
        let mut entity = object;
        while !self.is_area(entity) {
            entity = self.object_parent(entity)?;
        }
        Some(entity)
    }

    pub fn create_portal(&mut self, kind: Value, (sa, sb): (Entity, Entity)) -> Entity {
        assert!(self.is_space(sa));
        assert!(self.is_space(sb));
        let portal = self.spawn();
        self.layout.portals.insert(portal);
        self.layout.kinds.insert(portal, kind.clone());
        let oa = self.create_object(kind.clone(), sa);
        let ob = self.create_object(kind, sb);
        self.layout.portal_objects.insert(oa, PortalTarget { portal, target_object: ob });
        self.layout.portal_objects.insert(ob, PortalTarget { portal, target_object: oa });
        self.recalculate();
        portal
    }

    pub fn is_portal(&self, entity: Entity) -> bool {
        self.layout.portals.contains(&entity)
    }

    pub fn is_portal_object(&self, entity: Entity) -> bool {
        self.layout.portal_objects.contains_key(&entity)
    }

    pub fn object_portal(&self, object: Entity) -> Option<Entity> {
        self.layout.portal_objects.get(&object).map(|target| target.portal)
    }

    pub fn object_portal_target(&self, object: Entity) -> Option<Entity> {
        self.layout.portal_objects.get(&object).map(|target| target.target_object)
    }

    fn recalculate(&mut self) {
        self.recalculate_paths();
        self.recalculate_space_distances();
    }

    fn recalculate_space_distances(&mut self) {
        self.layout.space_distances.clear();
        for path in self.find_paths() {
            let mut spaces = path.into_iter()
                .map(|area| self.object_space(area).unwrap())
                .collect::<Vec<_>>();
            let key = Pair::new(*spaces.first().unwrap(), *spaces.last().unwrap());
            spaces.sort();
            spaces.dedup();
            if spaces.len() < 2 {
                continue;
            }
            self.layout.space_distances.entry(key)
                .and_modify(|current| *current = (*current).min(spaces.len()))
                .or_insert(spaces.len());
        }
    }

    fn recalculate_paths(&mut self) {
        self.layout.paths.clear();
        for path in self.find_paths() {
            let first = *path.first().unwrap();
            let last = *path.last().unwrap();
            self.layout.paths.entry((first, last)).or_default().push(
                path.iter().rev().copied().fold(
                    Value::List(Arc::new([])),
                    |prev, area| Value::List(Arc::new([Value::Ext(area), prev])),
                ),
            );
        }
    }

    fn find_paths(&self) -> Vec<Vec<Entity>> {
        let mut buffer = self.areas().map(|area| Vec::from([area])).collect::<VecDeque<_>>();
        let mut paths = Vec::new();

        while let Some(path) = buffer.pop_front() {
            if path.len() > 1 {
                paths.push(path.clone());
            }
            let last = *path.last().unwrap();
            let mut try_extend = |area| if !path.contains(&area) {
                let mut path = path.clone();
                path.push(area);
                buffer.push_back(path);
            };
            if let Some(target) = self.object_portal_target(last) {
                try_extend(target);
            }
            for local in self.child_objects(self.object_space(last).unwrap()) {
                try_extend(local);
            }
        }

        paths
    }
}
