// Replicated Growable Array (RGA) Implementation
// Based on "Replicated Abstract Data Types: Building Blocks for Collaborative Applications"
// by Roh et al., 2011

pub mod node;
pub mod remote_op;
pub mod rga;
pub mod s4vector;

pub use {node::Node, remote_op::RemoteOp, rga::Rga, s4vector::S4Vector};
