pub mod config;
mod domain;
pub mod email_client;
mod routes;
mod startup;
pub mod telemetry;

pub use startup::run;
