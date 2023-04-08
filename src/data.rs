use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use float_ord::FloatOrd;
use reagenz::ScriptSource;
use smol_str::SmolStr;
use src_ctx::{ContextError, SourceMap, LoadError, Origin, SourceError};
use treelang::{Item, Indent, ParseError, Tree, Node, ItemKind};

use crate::behavior::Value;


type Handler<T, E> = fn(Meta, Vec<Value>, Vec<Value>, Vec<T>) -> Result<T, E>;

type DataResult<T, E> = Result<T, DataError<E>>;
type FormatResult<T, E> = Result<T, SourceError<FormatError<E>>>;

#[derive(Debug, Clone, Default)]
pub struct Meta {
    pub global_attributes: HashMap<Value, Value>,
    pub global_tags: HashSet<Value>,
    pub agent_attributes: HashMap<SmolStr, HashMap<Value, Value>>,
    pub agent_tags: HashMap<SmolStr, HashSet<Value>>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DataError<E> {
    #[error(transparent)]
    Load(#[from] LoadError),
    #[error(transparent)]
    Parse(#[from] ContextError<ParseError>),
    #[error(transparent)]
    Format(#[from] ContextError<FormatError<E>>),
    #[error("Multiple definitions of named source `{name}`")]
    NamedSourceConflict { name: Arc<str> },
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum FormatError<E> {
    #[error(transparent)]
    Data(E),
    #[error("Invalid data element")]
    InvalidElement,
    #[error("Unknown element kind `{key}`")]
    UnknownElement { key: SmolStr },
    #[error("Invalid metadata")]
    InvalidMeta,
    #[error("Meta agent specifications have to be symbols")]
    InvalidMetaAgent,
    #[error("Statements are invalid at the top level")]
    TopLevelStatement,
    #[error("Invalid value")]
    InvalidValue,
}

#[derive(derivative::Derivative)]
#[derivative(Clone(bound=""), Default(bound=""))]
pub struct DataLoader<T, E> {
    parsers: HashMap<SmolStr, Handler<T, E>>,
}

impl<T, E> DataLoader<T, E> {
    pub fn register<N>(&mut self, name: N, handler: Handler<T, E>)
    where
        N: Into<SmolStr>,
    {
        self.parsers.insert(name.into(), handler);
    }

    pub fn load<I>(&self, indent: Indent, sources: I) -> DataResult<Vec<T>, E>
    where
        I: IntoIterator<Item = ScriptSource>,
    {
        let mut map = SourceMap::new();
        for source in sources {
            match source {
                ScriptSource::Path { path } => {
                    map.load_directory(path, ".data")?;
                },
                ScriptSource::Str { content, name } => {
                    map.insert(Origin::Named(name.clone()), content).try_into_inserted().ok()
                        .ok_or_else(|| DataError::NamedSourceConflict { name })?;
                },
            }
        }
        let mut elements = Vec::new();
        for index in map.origins().map(|origin| map.origin_index(origin).unwrap()) {
            let input = map.input(index);
            let tree = Tree::parse(input, indent)
                .map_err(|error| error.into_context_error(&map))?;
            for node in tree.roots {
                let element = self.parse(&node).map_err(|error| error.into_context_error(&map))?;
                elements.push(element);
            }
        }
        Ok(elements)
    }

    fn parse(&self, node: &Node) -> FormatResult<T, E> {
        let Some(dir) = node.directive() else {
            return Err(SourceError::new(
                FormatError::TopLevelStatement,
                node.location,
                "statement",
            ));
        };
        let Some((key, key_item, signature)) = extract_key(&dir.signature) else {
            return Err(SourceError::new(
                FormatError::InvalidElement,
                node.location,
                "expected data element",
            ));
        };
        let Some(handler) = self.parsers.get(key) else {
            return Err(SourceError::new(
                FormatError::UnknownElement { key: key.clone() },
                key_item.location.start(),
                "unknown data alement",
            ));
        };
        let signature = reify_values(signature)?;
        let arguments = reify_values(&dir.arguments)?;
        let mut children = Vec::new();
        let mut meta = Meta::default();
        for child in node.children() {
            if let Some(stmt) = node.statement() {
                let Some((key, _, arguments)) = extract_key(&stmt.signature) else {
                    return Err(SourceError::new(
                        FormatError::InvalidMeta,
                        child.location,
                        "expected meta element",
                    ));
                };
                match (key.as_str(), &arguments[..]) {
                    ("%", [tag]) => {
                        meta.global_tags.insert(reify(tag)?);
                    },
                    ("%", [name, value]) => {
                        meta.global_attributes
                            .insert(reify(name)?, reify(value)?);
                    },
                    ("%", _) => {
                        return Err(SourceError::new(
                            FormatError::InvalidMeta,
                            node.location,
                            "expected tag or key/value pair",
                        ));
                    },
                    ("@", [agent, tag]) => {
                        let Some(agent) = agent.word() else {
                            return Err(SourceError::new(
                                FormatError::InvalidMetaAgent,
                                agent.location.start(),
                                "expected agent symbol",
                            ));
                        };
                        meta.agent_tags
                            .entry(agent.clone())
                            .or_default()
                            .insert(reify(tag)?);
                    },
                    ("@", [agent, name, value]) => {
                        let Some(agent) = agent.word() else {
                            return Err(SourceError::new(
                                FormatError::InvalidMetaAgent,
                                agent.location.start(),
                                "expected agent symbol",
                            ));
                        };
                        meta.agent_attributes
                            .entry(agent.clone())
                            .or_default()
                            .insert(reify(name)?, reify(value)?);
                    },
                    ("@", _) => {
                        return Err(SourceError::new(
                            FormatError::InvalidMeta,
                            node.location,
                            "expected agent followed by tag or key/value pair",
                        ));
                    },
                    _ => {
                        return Err(SourceError::new(
                            FormatError::InvalidMeta,
                            node.location,
                            "expected meta element",
                        ));
                    },
                }
                todo!()
            } else {
                children.push(self.parse(child)?);
            }
        }
        handler(meta, signature, arguments, children)
            .map_err(FormatError::Data)
            .map_err(|error| SourceError::new(error, node.location, "data error occured here"))
    }
}

fn reify_values<E>(items: &[Item]) -> FormatResult<Vec<Value>, E> {
    let mut values = Vec::new();
    for item in items {
        values.push(reify(item)?);
    }
    Ok(values)
}

fn reify<E>(item: &Item) -> FormatResult<Value, E> {
    match &item.kind {
        ItemKind::Word(word) => Ok(Value::Symbol(word.clone())),
        ItemKind::Int(value) => Ok(Value::Int(*value)),
        ItemKind::Float(value) => Ok(Value::Float(FloatOrd(*value))),
        ItemKind::Parentheses(values) => Ok(Value::List(reify_values(values)?.into())),
        _ => Err(SourceError::new(
            FormatError::InvalidValue,
            item.location.start(),
            "expected value",
        )),
    }
}

fn extract_key(items: &[Item]) -> Option<(&SmolStr, &Item, &[Item])> {
    let (key, rest) = items.split_first()?;
    let word = key.word()?;
    Some((word, key, rest))
}