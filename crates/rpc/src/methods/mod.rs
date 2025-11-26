//! RPC method namespaces. Each module mirrors the TS SDK methods with
//! thin wrappers around `RpcClient`.

pub mod avatar;
pub mod balance;
pub mod events;
pub mod group;
pub mod health;
pub mod invitation;
pub mod network;
pub mod pathfinder;
pub mod query;
pub mod search;
pub mod tables;
pub mod token;
pub mod token_info;
pub mod trust;

pub use avatar::AvatarMethods;
pub use balance::BalanceMethods;
pub use events::EventsMethods;
pub use group::GroupMethods;
pub use health::HealthMethods;
pub use invitation::InvitationMethods;
pub use network::NetworkMethods;
pub use pathfinder::PathfinderMethods;
pub use query::QueryMethods;
pub use search::SearchMethods;
pub use tables::TablesMethods;
pub use token::TokenMethods;
pub use token_info::TokenInfoMethods;
pub use trust::TrustMethods;
