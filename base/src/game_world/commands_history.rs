use std::{
    collections::LinkedList,
    sync::atomic::{AtomicU8, Ordering},
};

use bevy::{
    ecs::{
        entity::{EntityHashMap, MapEntities},
        system::SystemParam,
    },
    prelude::*,
};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::GameState;

pub(super) struct CommandHistoryPlugin;

impl Plugin for CommandHistoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HistoryBuffer>()
            .init_resource::<CommandIds>()
            .add_server_event::<CommandConfirmation>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::confirm
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(OnExit(GameState::InGame), Self::cleanup);
    }
}

impl CommandHistoryPlugin {
    fn confirm(
        mut commands: Commands,
        mut confirmation_events: EventReader<CommandConfirmation>,
        mut buffer: ResMut<HistoryBuffer>,
        despawn_entities: Query<(Entity, &PendingDespawn)>,
    ) {
        for &confirmation in confirmation_events.read() {
            buffer.confirm(confirmation);

            if let Some((entity, _)) = despawn_entities
                .iter()
                .find(|(_, despawn)| despawn.command_id == confirmation.id)
            {
                debug!("despawning entity `{entity}` for `{confirmation:?}`");
                commands.entity(entity).despawn_recursive();
            }
        }
    }

    fn cleanup(mut buffer: ResMut<HistoryBuffer>) {
        buffer.clear();
    }
}

/// Entities marked with this component will be despawned when the command with this ID will be confirmed.
#[derive(Component)]
pub(super) struct PendingDespawn {
    pub(super) command_id: CommandId,
}

#[derive(SystemParam)]
pub struct CommandsHistory<'w, 's> {
    commands: Commands<'w, 's>,
    ids: Res<'w, CommandIds>,
}

impl CommandsHistory<'_, '_> {
    /// Like [`Commands::push`], but can be reverted with [`Self::undo`].
    #[allow(dead_code)]
    pub(super) fn push<C: ReversibleCommand + 'static>(&mut self, command: C) {
        self.commands.queue(move |world: &mut World| {
            world.resource_scope(|world, mut buffer: Mut<HistoryBuffer>| {
                buffer.apply(
                    Box::new(command),
                    Vec::new(),
                    Stack::Undo { new: true },
                    world,
                );
            })
        });
    }

    /// Like [`Self::push`], but the command will wait for confirmation from server to be available for [`Self::undo`].
    ///
    /// See also [`CommandConfirmation`].
    pub(super) fn push_pending<C: PendingCommand + 'static>(&mut self, command: C) -> CommandId {
        let id = self.ids.next();
        self.commands.queue(move |world: &mut World| {
            world.resource_scope(|world, mut buffer: Mut<HistoryBuffer>| {
                buffer.apply_pending(
                    id,
                    Box::new(command),
                    Vec::new(),
                    Stack::Undo { new: true },
                    world,
                );
            })
        });

        id
    }

    /// Reverses the last executed command if exists.
    pub fn undo(&mut self) {
        self.commands.queue(|world: &mut World| {
            world.resource_scope(|world, mut buffer: Mut<HistoryBuffer>| {
                buffer.apply_reverse(Stack::Redo, world);
            })
        });
    }

    /// Re-applies the last undone command if exists.
    pub fn redo(&mut self) {
        self.commands.queue(|world: &mut World| {
            world.resource_scope(|world, mut buffer: Mut<HistoryBuffer>| {
                buffer.apply_reverse(Stack::Undo { new: false }, world);
            })
        });
    }
}

/// ID generator for pending commands.
///
/// We use a wrapping [`u8`] because realistically users will have
/// only a few unconfirmed commands at a time.
///
/// We utilize interior mutability to let systems that schedule commands in run parallel.
#[derive(Resource, Default)]
struct CommandIds(AtomicU8);

impl CommandIds {
    /// Generates a new ID for a command.
    fn next(&self) -> CommandId {
        CommandId(self.0.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Resource, Default)]
struct HistoryBuffer {
    undo: LinkedList<CommandRecord>,
    redo: LinkedList<CommandRecord>,
    mapper: CommandEntityMapper,
    unconfirmed: Vec<UnconfirmedCommand>,
}

impl HistoryBuffer {
    /// Applies the command for `stack` from the reverse one.
    fn apply_reverse(&mut self, stack: Stack, world: &mut World) {
        let record = match stack {
            Stack::Undo { .. } => self.redo.pop_back(),
            Stack::Redo => self.undo.pop_back(),
        };

        if let Some(record) = record {
            match record.command {
                ReverseCommand::Reversible(command) => {
                    self.apply(command, record.entities, stack, world)
                }
                ReverseCommand::Pending(command) => {
                    let id = world.resource::<CommandIds>().next();
                    self.apply_pending(id, command, record.entities, stack, world);
                }
            }
        }
    }

    fn apply(
        &mut self,
        command: Box<dyn ReversibleCommand>,
        mut entities: Vec<Entity>,
        stack: Stack,
        world: &mut World,
    ) {
        debug!("applying command for `{stack:?}`");

        let command = self.record(&mut entities, |recorder| command.apply(recorder, world));
        let record = CommandRecord {
            command: ReverseCommand::Reversible(command),
            entities,
        };
        self.push(record, stack);
    }

    fn apply_pending(
        &mut self,
        id: CommandId,
        command: Box<dyn PendingCommand>,
        mut entities: Vec<Entity>,
        stack: Stack,
        world: &mut World,
    ) {
        debug!("applying pending command for `{stack:?}`");

        let command = self.record(&mut entities, |recorder| command.apply(id, recorder, world));
        self.unconfirmed.push(UnconfirmedCommand {
            id,
            stack,
            entities,
            command,
        });
    }

    /// Confirms a command added by [`Self::apply_pending`].
    fn confirm(&mut self, confirmation: CommandConfirmation) {
        if let Some(index) = self
            .unconfirmed
            .iter()
            .position(|unconfirmed| unconfirmed.id == confirmation.id)
        {
            debug!("applying `{confirmation:?}`");
            let mut unconfirmed = self.unconfirmed.swap_remove(index);
            let command = self.record(&mut unconfirmed.entities, |recorder| {
                unconfirmed.command.confirm(recorder, confirmation)
            });
            let record = CommandRecord {
                command: ReverseCommand::Pending(command),
                entities: unconfirmed.entities,
            };
            self.push(record, unconfirmed.stack);
        } else {
            debug!("ignoring `{confirmation:?}`");
        }
    }

    fn push(&mut self, record: CommandRecord, stack: Stack) {
        match stack {
            Stack::Undo { new } => {
                self.undo.push_back(record);

                const HISTORY_LEN: usize = 25;
                if self.undo.len() > HISTORY_LEN {
                    self.undo.pop_front();
                }

                if new {
                    // Clear all redo commands on a new command.
                    self.redo.clear();
                    self.unconfirmed
                        .retain(|command| matches!(command.stack, Stack::Undo { .. }));
                }
            }
            Stack::Redo => self.redo.push_back(record),
        }
    }

    /// Records entities changed in `f` and updates them inside commands.
    fn record<C>(&mut self, entities: &mut Vec<Entity>, f: impl FnOnce(EntityRecorder) -> C) -> C {
        let recorder = EntityRecorder::new(entities, &mut self.mapper);
        let command = (f)(recorder);

        if !self.mapper.is_empty() {
            for record in self.undo.iter_mut().chain(&mut self.redo) {
                match &mut record.command {
                    ReverseCommand::Reversible(command) => {
                        command.map_command_entities(&mut self.mapper)
                    }
                    ReverseCommand::Pending(command) => {
                        command.map_command_entities(&mut self.mapper)
                    }
                }
            }
            debug!("updated {} entities inside commands", self.mapper.len());
            self.mapper.clear();
        }

        command
    }

    fn clear(&mut self) {
        debug!("clearing history buffer");
        self.undo.clear();
        self.redo.clear();
        self.unconfirmed.clear();
    }
}

/// Regular or confirmed command.
struct CommandRecord {
    command: ReverseCommand,
    /// Entities produced by the command.
    entities: Vec<Entity>,
}

enum ReverseCommand {
    Reversible(Box<dyn ReversibleCommand>),
    Pending(Box<dyn PendingCommand>),
}

/// Command that waits for confirmation from server.
struct UnconfirmedCommand {
    id: CommandId,
    /// State when the command were executed.
    stack: Stack,
    /// Entities produced by the command.
    entities: Vec<Entity>,
    command: Box<dyn ConfirmableCommand>,
}

/// Stack state.
#[derive(Debug, Clone, Copy)]
enum Stack {
    Undo { new: bool },
    Redo,
}

impl Default for Stack {
    fn default() -> Self {
        Self::Undo { new: true }
    }
}

/// Like [`Command`](bevy::ecs::world::Command), but can be reversed.
pub(super) trait ReversibleCommand: MapCommandEntities + Send + Sync {
    fn apply(
        self: Box<Self>,
        recorder: EntityRecorder,
        world: &mut World,
    ) -> Box<dyn ReversibleCommand>;
}

/// Like [`ReversibleCommand`], but also requires confirmation from server to be considered applied.
pub(super) trait PendingCommand: MapCommandEntities + Send + Sync {
    fn apply(
        self: Box<Self>,
        id: CommandId,
        recorder: EntityRecorder,
        world: &mut World,
    ) -> Box<dyn ConfirmableCommand>;
}

/// Command that needs confirmation from server.
pub(super) trait ConfirmableCommand: Send + Sync {
    /// Transforms an uncofirmed command into a command that can be used with undo/redo.
    ///
    /// Needed for commands that require additional information from server.
    fn confirm(
        self: Box<Self>,
        recorder: EntityRecorder,
        confirmation: CommandConfirmation,
    ) -> Box<dyn PendingCommand>;
}

/// Records entity changes in commands.
///
/// Needed to correctly handle entity references in commands that spawn/despawn entities.
/// Entities tracked by their record order using internal indexing.
/// The index resets each command execution.
pub(super) struct EntityRecorder<'a> {
    index: usize,
    entities: &'a mut Vec<Entity>,
    mapper: &'a mut CommandEntityMapper,
}

impl<'a> EntityRecorder<'a> {
    fn new(entities: &'a mut Vec<Entity>, mapper: &'a mut CommandEntityMapper) -> Self {
        Self {
            index: 0,
            entities,
            mapper,
        }
    }

    /// Record command entity that may change during the undo/redo.
    pub(super) fn record(&mut self, entity: Entity) {
        if let Some(old_entity) = self.entities.get_mut(self.index) {
            if *old_entity != entity {
                trace!("mapping `{old_entity}` to `{entity}`");
                self.mapper.insert(*old_entity, entity);
                *old_entity = entity;
            }
        } else {
            trace!("recording `{entity}`");
            self.entities.push(entity);
        }

        self.index += 1;
    }
}

/// Server event to notify client about command confirmation.
#[derive(Event, Serialize, Deserialize, Clone, Copy, Debug)]
pub(super) struct CommandConfirmation {
    /// Confirmed command ID.
    pub(super) id: CommandId,

    /// Associated entity.
    ///
    /// Needed for some commands to properly generate the undo/redo.
    pub(super) entity: Option<Entity>,
}

impl CommandConfirmation {
    /// Creates a new confirmation without an associated entity.
    pub(super) fn new(id: CommandId) -> Self {
        Self { id, entity: None }
    }
}

/// ID for an unconfirmed command.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) struct CommandId(u8);

#[derive(Deref, DerefMut, Default)]
pub(super) struct CommandEntityMapper(EntityHashMap<Entity>);

impl EntityMapper for CommandEntityMapper {
    fn map_entity(&mut self, entity: Entity) -> Entity {
        *self.0.get(&entity).unwrap_or(&entity)
    }
}

/// Helper to use [`MapEntities`] with [`CommandEntityMapper`].
///
/// Needed because [`MapEntities`] is generic over the mapper and can't be made into an object.
pub(super) trait MapCommandEntities {
    fn map_command_entities(&mut self, mapper: &mut CommandEntityMapper);
}

impl<T: MapEntities> MapCommandEntities for T {
    fn map_command_entities(&mut self, mapper: &mut CommandEntityMapper) {
        self.map_entities(mapper)
    }
}

/// Generic event to send a pending command with its ID.
///
/// Event should be registered for each command.
#[derive(Event, Clone, Copy, Serialize, Deserialize)]
pub(super) struct CommandRequest<C> {
    pub(super) id: CommandId,
    pub(super) command: C,
}

impl<C: MapEntities> MapEntities for CommandRequest<C> {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.command.map_entities(entity_mapper);
    }
}
