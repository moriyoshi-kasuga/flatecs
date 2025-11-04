// This module provides common test structures that can be used by test code.
// Doctests define their own structs inline for clarity and self-containment.
#![allow(dead_code)]

use crate as structecs;
use structecs::*;

#[derive(Debug, Extractable)]
pub struct Entity {
    pub name: String,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Player {
    pub entity: Entity,
    pub health: u32,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Monster {
    pub entity: Entity,
    pub damage: u32,
}

#[derive(Debug, Extractable)]
pub struct Buff {
    pub power: u32,
}
