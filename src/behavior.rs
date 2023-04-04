use std::sync::Arc;

use once_cell::unsync::OnceCell;
use reagenz::{BehaviorTree, ScriptSource, ScriptError, BehaviorTreeBuilder, query_fn};
use treelang::Indent;

use crate::world::World;
use crate::world::entities::Entity;


pub type Value = reagenz::Value<Entity>;
pub type Values = reagenz::Values<Entity>;

pub struct Behavior<'a> {
    tree: BehaviorTree<Context<'a>, Entity, Effect>,
}

impl<'a> Behavior<'a> {
    pub fn load<'i, I>(indent: Indent, sources: I) -> Result<Self, ScriptError>
    where
        I: IntoIterator<Item = ScriptSource<'i>>,
    {
        let mut tree = BehaviorTreeBuilder::default();
        setup_tree_globals(&mut tree);
        setup_tree_queries(&mut tree);
        let tree = tree.compile(indent, sources)?;
        Ok(Self { tree })
    }
}

fn setup_tree_queries(tree: &mut BehaviorTreeBuilder<Context<'_>, Entity, Effect>) {
    tree.register_query("spaces", query_fn!(ctx => ctx.spaces().map(Value::Ext)));
}

fn setup_tree_globals(tree: &mut BehaviorTreeBuilder<Context<'_>, Entity, Effect>) {
    tree.register_global("$^self", |ctx| Value::Ext(ctx.agent));
    tree.register_global("$^location", |ctx| Value::Ext(ctx.location()));
    tree.register_global("$^space", |ctx| Value::Ext(ctx.space()));
    tree.register_global("$^position", |ctx| {
        let position = ctx.world.agent_position(ctx.agent).expect("invalid context position");
        position.clone()
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
}

#[derive(Debug, Clone)]
struct Context<'a> {
    agent: Entity,
    world: &'a World,
    spaces: OnceCell<Arc<[Entity]>>,
}

impl<'a> Context<'a> {
    fn new(world: &'a World, agent: Entity) -> Self {
        Self {
            world,
            agent,
            spaces: OnceCell::new(),
        }
    }

    fn location(&self) -> Entity {
        self.world.agent_location(self.agent).expect("context agent location")
    }

    fn space(&self) -> Entity {
        self.world.object_space(self.location()).expect("context agent space")
    }

    fn spaces(&self) -> impl Iterator<Item = Entity> + '_ {
        self.spaces.get_or_init(|| {
            self.world.spaces_by_distance(self.space()).collect()
        }).iter().copied()
    }
}
