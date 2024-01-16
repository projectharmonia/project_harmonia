use bevy_rapier3d::prelude::*;

/// Meaningful aliases for groups.
pub(super) trait HarmoniaGroupsExt {
    const GROUND: Self;
    const ACTOR: Self;
    const OBJECT: Self;
    const WALL: Self;
}

impl HarmoniaGroupsExt for Group {
    const GROUND: Self = Group::GROUP_1;
    const ACTOR: Self = Group::GROUP_2;
    const OBJECT: Self = Group::GROUP_3;
    const WALL: Self = Group::GROUP_4;
}
