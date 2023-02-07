use bevy::prelude::*;

/// A condition for systems to check if any component of type `T` exists in the world.
pub(crate) const fn any_component_exists<T: Component>() -> impl Fn(Query<(), With<T>>) -> bool {
    move |components: Query<(), With<T>>| -> bool { !components.is_empty() }
}
