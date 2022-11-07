pub use prost::bytes;
pub use prost::Message;

pub mod plugin {
    include!(concat!(env!("OUT_DIR"), "/plugin.rs"));
}
