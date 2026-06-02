//! Gateway WebSocket protocol client types.
//! Keep `PROTOCOL_VERSION` in sync with `packages/gateway-protocol/src/version.ts`.

pub mod client;
pub mod device_auth;
pub mod frames;
pub mod version;

pub use client::{ConnectParams, GatewayClient, GatewayClientConfig, GatewayRole};
pub use frames::{EventFrame, GatewayFrame, RequestFrame, ResponseFrame};
pub use version::{MIN_CLIENT_PROTOCOL_VERSION, PROTOCOL_VERSION};
