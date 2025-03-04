use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use kesko_types::resource::KeskoRes;

use crate::rapier_extern::rapier;

/// Component to indicate if an entity should generate collision events
#[derive(Component)]
pub struct GenerateCollisionEvents;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CollisionEvent {
    CollisionStarted(CollisionData),
    CollisionStopped(CollisionData),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollisionData {
    pub entity1: Entity,
    pub entity2: Entity,
    pub flag: rapier::geometry::CollisionEventFlags,
}

// Responsible for fetching collision events from Rapier and propagate them to Bevy
#[derive(Resource)]
pub(crate) struct CollisionEventHandler {
    collision_send: crossbeam::channel::Sender<rapier::geometry::CollisionEvent>,
    collision_recv: crossbeam::channel::Receiver<rapier::geometry::CollisionEvent>,
}

impl rapier::pipeline::EventHandler for CollisionEventHandler {
    /// fetches the collision events from rapier and sending them through the crossbeam
    fn handle_collision_event(
        &self,
        _bodies: &rapier::dynamics::RigidBodySet,
        _colliders: &rapier::geometry::ColliderSet,
        event: rapier::geometry::CollisionEvent,
        _contact_pair: Option<&rapier::geometry::ContactPair>,
    ) {
        if let Err(e) = self.collision_send.send(event) {
            error!("Failed to propagate collision event: {e}");
        }
    }

    fn handle_contact_force_event(
        &self,
        _dt: rapier::math::Real,
        _bodies: &rapier::dynamics::RigidBodySet,
        _colliders: &rapier::geometry::ColliderSet,
        _contact_pair: &rapier::geometry::ContactPair,
        _total_force_magnitude: rapier::math::Real,
    ) {
        todo!("Implement when needed");
    }
}

impl CollisionEventHandler {
    pub(crate) fn new() -> Self {
        let (send, recv) = crossbeam::channel::unbounded();
        Self {
            collision_send: send,
            collision_recv: recv,
        }
    }

    /// Propagate collision events from Rapier to Bevys event system
    fn send_events(
        &self,
        event_writer: &mut EventWriter<CollisionEvent>,
        colliders: &rapier::geometry::ColliderSet,
    ) {
        while let Ok(event) = self.collision_recv.try_recv() {
            match event {
                rapier::geometry::CollisionEvent::Started(handle1, handle2, flag) => {
                    if let (Some(coll1), Some(coll2)) =
                        (colliders.get(handle1), colliders.get(handle2))
                    {
                        event_writer.send(CollisionEvent::CollisionStarted(CollisionData {
                            entity1: Entity::from_bits(coll1.user_data as u64),
                            entity2: Entity::from_bits(coll2.user_data as u64),
                            flag,
                        }))
                    }
                }
                rapier::geometry::CollisionEvent::Stopped(handle1, handle2, flag) => {
                    if let (Some(coll1), Some(coll2)) =
                        (colliders.get(handle1), colliders.get(handle2))
                    {
                        event_writer.send(CollisionEvent::CollisionStopped(CollisionData {
                            entity1: Entity::from_bits(coll1.user_data as u64),
                            entity2: Entity::from_bits(coll2.user_data as u64),
                            flag,
                        }))
                    }
                }
            }
        }
    }
}

/// System for propagating collision events to Bevy from Rapier
/// Should be executed after the rapier pipeline step in order to capture the latest events
pub(crate) fn send_collision_events_system(
    colliders: Res<KeskoRes<rapier::geometry::ColliderSet>>,
    collision_event_manager: Res<CollisionEventHandler>,
    mut event_writer: EventWriter<CollisionEvent>,
) {
    collision_event_manager.send_events(&mut event_writer, &colliders);
}
