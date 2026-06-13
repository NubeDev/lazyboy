//! Membership: user/group identities, space membership, and feed
//! visibility (SCOPE.md "Feeds, membership, and visibility"). The first
//! structure past single-tenancy. These verbs MODEL the structure; per
//! R4 they are deliberately NOT wired into the MVP trust gate until
//! promoted — see DOCS/WORKFLOWS.md.

mod add_member;
mod create_group;
mod grant_membership;
mod list_groups;
mod list_members;
mod list_memberships;
mod list_visibility;
mod set_feed_visibility;

pub use add_member::add_member;
pub use create_group::create_group;
pub use grant_membership::grant_membership;
pub use list_groups::list_groups;
pub use list_members::list_members;
pub use list_memberships::list_memberships;
pub use list_visibility::list_visibility;
pub use set_feed_visibility::set_feed_visibility;
