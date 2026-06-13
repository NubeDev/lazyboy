//! Zero-size marker types parameterising `Id<T>`. They exist only to
//! make identifiers of different aggregates distinct at compile time.

#[derive(Clone, Copy, Debug)]
pub enum Workspace {}
#[derive(Clone, Copy, Debug)]
pub enum Space {}
#[derive(Clone, Copy, Debug)]
pub enum Message {}
#[derive(Clone, Copy, Debug)]
pub enum Task {}
#[derive(Clone, Copy, Debug)]
pub enum AgentRun {}
#[derive(Clone, Copy, Debug)]
pub enum Approval {}
#[derive(Clone, Copy, Debug)]
pub enum Identity {}
