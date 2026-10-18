mod ipv6;
mod names;
mod routes;
mod wake;

pub use ipv6::{Ipv6, set as set_ipv6, status as ipv6_status};
pub use names::{LocalName, LocalNameInput, create as create_name, delete as delete_name, domain, list as names};
pub use routes::{Route, RouteInput, create as create_route, delete as delete_route, list as routes};
pub use wake::wake;
