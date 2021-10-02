use amethyst::core::bundle::SystemBundle;
use amethyst::ecs::*;
use amethyst::error::Error;
use amethyst::{
    assets::AssetStorage,
    audio::{output::Output, Source, SourceHandle},
    core::transform::{components::Parent, Transform},
};
use nalgebra::geometry::{Isometry2, Point2, Point3, UnitQuaternion};
use nalgebra::{RealField, Vector2};
use ncollide2d::pipeline::narrow_phase::ContactEvent;
use ncollide2d::pipeline::object::CollisionGroups;
use ncollide2d::query::{Proximity, Ray};
use nphysics2d::force_generator::DefaultForceGeneratorSet;
use nphysics2d::joint::DefaultJointConstraintSet;
use nphysics2d::math::{Force, ForceType};
use nphysics2d::object::*;
use nphysics2d::world::{
    DefaultGeometricalWorld, DefaultMechanicalWorld, GeometricalWorld, MechanicalWorld,
};

type N = f32;

#[derive(Component)]
#[storage(VecStorage)]
pub struct PhysicsDesc {
    body: RigidBodyDesc<f32>,
    collider: ColliderDesc<f32>,
}

impl PhysicsDesc {
    pub fn new(body: RigidBodyDesc<f32>, collider: ColliderDesc<f32>) -> Self {
        PhysicsDesc { body, collider }
    }
}

#[derive(Component)]
#[storage(VecStorage)]
pub struct AttachedSensor {
    collider: ColliderDesc<f32>,
    pub handle: Option<(DefaultBodyHandle, DefaultColliderHandle)>,
    fresh: bool,
}

impl AttachedSensor {
    pub fn new(collider: ColliderDesc<f32>) -> Self {
        AttachedSensor {
            collider,
            handle: None,
            fresh: false,
        }
    }
    pub fn set_handle(&mut self, handle: (DefaultBodyHandle, DefaultColliderHandle)) {
        self.handle = Some(handle);
        self.fresh = true;
    }
    pub fn get_handle(&self) -> PhysicsHandle {
        if let Some(handle) = self.handle {
            PhysicsHandle {
                body: Some(handle.0),
                collider: Some(handle.1),
                fresh: false,
            }
        } else {
            PhysicsHandle {
                body: None,
                collider: None,
                fresh: true,
            }
        }
    }
}

#[derive(Component, Debug, Clone)]
#[storage(VecStorage)]
pub struct PhysicsHandle {
    body: Option<DefaultBodyHandle>,
    collider: Option<DefaultColliderHandle>,
    fresh: bool,
}

impl PhysicsHandle {
    pub fn new(b: DefaultBodyHandle, c: DefaultColliderHandle) -> Self {
        Self {
            body: Some(b),
            collider: Some(c),
            fresh: true,
        }
    }
}

pub struct Physics {
    pub geo_world: DefaultGeometricalWorld<N>,
    pub mech_world: DefaultMechanicalWorld<N>,
    pub bodies: DefaultBodySet<N>,
    pub colliders: DefaultColliderSet<N>,
    pub joint_constraints: DefaultJointConstraintSet<N>,
    pub force_generators: DefaultForceGeneratorSet<N>,
}

impl Physics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self) {
        self.mech_world.step(
            &mut self.geo_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        )
    }

    pub fn spawn(
        &mut self,
        body_desc: &RigidBodyDesc<N>,
        collider_desc: &ColliderDesc<N>,
    ) -> (DefaultBodyHandle, DefaultColliderHandle) {
        let handle = self.bodies.insert(body_desc.build());
        let collider_handle = self
            .colliders
            .insert(collider_desc.build(BodyPartHandle(handle, 0)));
        (handle, collider_handle)
    }

    pub fn add_child_collider(
        &mut self,
        parent_handle: &PhysicsHandle,
        collider_desc: &ColliderDesc<N>,
    ) -> DefaultColliderHandle {
        if let Some(handle) = parent_handle.body {
            let collider_handle = self
                .colliders
                .insert(collider_desc.build(BodyPartHandle(handle, 0)));
            collider_handle
        } else {
            panic!("No parent!");
        }
    }

    pub fn get_position(&self, handle: &PhysicsHandle) -> Option<Isometry2<N>> {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body(handle) {
                Some(rigid_body.position().clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_location(&self, handle: &PhysicsHandle) -> Option<Point2<N>> {
        self.get_position(handle)
            .map(|location| location.translation.vector)
            .map(|vector| Point2::new(vector.x, vector.y))
    }

    pub fn set_location(&mut self, handle: &PhysicsHandle, x: N, y: N) {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body_mut(handle) {
                rigid_body.set_position(Isometry2::new(
                    Vector2::new(x, y),
                    rigid_body.position().rotation.angle(),
                ));
            }
        }
    }

    pub fn set_sensor_position(&mut self, sensor: &AttachedSensor, x: N, y: N) {
        if let Some(handle) = sensor.handle {
            if let (Some(parent), Some(collider)) = (
                self.bodies.rigid_body(handle.0),
                self.colliders.get_mut(handle.1),
            ) {
                collider.set_position(Isometry2::new(
                    parent.position().translation.vector + Vector2::new(x, y),
                    collider.position().rotation.angle(),
                ));
            }
        }
    }

    pub fn set_rotation(&mut self, handle: &PhysicsHandle, radians: N) {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body_mut(handle) {
                rigid_body.set_position(Isometry2::new(
                    rigid_body.position().translation.vector,
                    radians,
                ));
            }
        }
    }

    pub fn get_velocity(&mut self, handle: &PhysicsHandle) -> Option<Vector2<N>> {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body(handle) {
                return Some(rigid_body.velocity().linear);
            }
        }
        None
    }

    pub fn set_velocity(&mut self, handle: &PhysicsHandle, vec: Vector2<N>) {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body_mut(handle) {
                rigid_body.set_linear_velocity(vec);
            }
        }
    }

    pub fn set_damping(&mut self, handle: &PhysicsHandle, damping: N) {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body_mut(handle) {
                rigid_body.set_linear_damping(damping);
            }
        }
    }

    pub fn set_angular_velocity(&mut self, handle: &PhysicsHandle, rotational: N) {
        if let Some(handle) = handle.body {
            if let Some(rigid_body) = self.bodies.rigid_body_mut(handle) {
                rigid_body.set_angular_velocity(rotational);
            }
        }
    }

    pub fn apply_dampening(&mut self, handle: &PhysicsHandle, mag: N) {
        if let Some(current_vel) = self.get_velocity(handle) {
            if let Some(handle) = handle.body {
                if let Some(body) = self.bodies.get_mut(handle) {
                    body.apply_force(
                        0,
                        &Force::linear(-current_vel * mag / (1.0 + mag)),
                        ForceType::AccelerationChange,
                        true,
                    );
                }
            }
        }
    }

    pub fn apply_velocity_change(&mut self, handle: &PhysicsHandle, vec: Vector2<N>) {
        if let Some(handle) = handle.body {
            if let Some(body) = self.bodies.get_mut(handle) {
                body.apply_force(0, &Force::linear(vec), ForceType::VelocityChange, true)
            } else {
                panic!("HOLY SHIT");
            }
        } else {
            panic!("NOLY SHIT");
        }
    }

    pub fn apply_force(&mut self, handle: &PhysicsHandle, vec: Vector2<N>) {
        if let Some(handle) = handle.body {
            if let Some(body) = self.bodies.get_mut(handle) {
                body.apply_force(0, &Force::linear(vec), ForceType::Force, true);
            }
        }
    }

    pub fn apply_impulse(&mut self, handle: &PhysicsHandle, vec: Vector2<N>) {
        if let Some(handle) = handle.body {
            if let Some(body) = self.bodies.get_mut(handle) {
                body.apply_force(0, &Force::linear(vec), ForceType::Impulse, true)
            }
        }
    }

    pub fn get_body_entity(&self, handle: DefaultBodyHandle) -> Option<&Entity> {
        if let Some(rigid_body) = self.bodies.rigid_body(handle) {
            if let Some(m_entity) = rigid_body.user_data() {
                m_entity.downcast_ref::<Entity>()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_collider_entity(&self, handle: DefaultColliderHandle) -> Option<&Entity> {
        if let Some(collider) = self.colliders.get(handle) {
            if let Some(m_entity) = collider.user_data() {
                m_entity.downcast_ref::<Entity>()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_between(
        &self,
        handle1: &PhysicsHandle,
        handle2: &PhysicsHandle,
    ) -> Option<Vector2<N>> {
        if let (Some(collider1), Some(collider2)) = (handle1.collider, handle2.collider) {
            if let (Some(collider1), Some(collider2)) =
                (self.colliders.get(collider1), self.colliders.get(collider2))
            {
                Some(
                    collider2.position().translation.vector
                        - collider1.position().translation.vector,
                )
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn is_intersecting(&self, handle1: &PhysicsHandle, handle2: &PhysicsHandle) -> bool {
        if let (Some(collider1), Some(collider2)) = (handle1.collider, handle2.collider) {
            !self
                .geo_world
                .proximity_pair(&self.colliders, collider1, collider2, true)
                .is_none()
        } else {
            false
        }
    }

    pub fn get_intersections(&self, handle: &PhysicsHandle) -> Vec<Entity> {
        let mut found = Vec::new();
        if let Some(collider) = handle.collider {
            if let Some(interferences) = self
                .geo_world
                .colliders_in_proximity_of(&self.colliders, collider)
            {
                for interference in interferences {
                    if let Some(entity) = self.get_collider_entity(interference.0) {
                        found.push(*entity);
                    }
                }
            }
        }
        found
    }

    pub fn ray_cast(
        &self,
        handle: &PhysicsHandle,
        direction: Vector2<N>,
        groups: Option<CollisionGroups>,
    ) -> Vec<(Entity, N)> {
        let mut found = Vec::new();
        if let Some(center) = self.get_location(handle) {
            for interference in self.geo_world.interferences_with_ray(
                &self.colliders,
                &Ray::<N>::new(center, direction),
                500.0,
                &groups.unwrap_or_default(),
            ) {
                if let Some(entity) = self.get_collider_entity(interference.0) {
                    found.push((*entity, interference.2.toi));
                }
            }
        }
        found
    }
}

impl Default for Physics {
    fn default() -> Self {
        let mut mech_world = DefaultMechanicalWorld::new(Vector2::new(0.0, 0.0));
        // mech_world.set_timestep(N::from_f32(1.0 / 30.0).unwrap());
        Self {
            mech_world,
            geo_world: DefaultGeometricalWorld::new(),
            bodies: DefaultBodySet::new(),
            colliders: DefaultColliderSet::new(),
            joint_constraints: DefaultJointConstraintSet::new(),
            force_generators: DefaultForceGeneratorSet::new(),
        }
    }
}

struct PhysicsSystem;

impl<'s> System<'s> for PhysicsSystem {
    type SystemData = (
        Write<'s, Physics>,
        ReadStorage<'s, PhysicsHandle>,
        WriteStorage<'s, Transform>,
    );

    fn run(&mut self, (mut physics, handles, mut transforms): Self::SystemData) {
        physics.step();
        for (handle, transform) in (&handles, &mut transforms).join() {
            if let Some(position) = physics.get_position(handle) {
                let x = position.translation.x;
                let y = position.translation.y;
                let rotation_2d = position.rotation.angle();
                transform.set_translation_x(x as f32);
                transform.set_translation_y(y as f32);
                transform.set_rotation_2d(rotation_2d as f32);
            }
        }
    }
}

struct PhysicsSpawningSystem;

impl<'s> System<'s> for PhysicsSpawningSystem {
    type SystemData = (
        Write<'s, Physics>,
        ReadStorage<'s, PhysicsDesc>,
        WriteStorage<'s, AttachedSensor>,
        ReadStorage<'s, Parent>,
        WriteStorage<'s, PhysicsHandle>,
        WriteStorage<'s, Transform>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (mut physics, descs, mut attached, parent, mut handles, mut transforms, entities): Self::SystemData,
    ) {
        for (entity, desc) in (&entities, &descs).join() {
            if !handles.contains(entity) {
                let (handle, collider_handle) = physics.spawn(&desc.body, &desc.collider);
                let phys_handle = PhysicsHandle::new(handle, collider_handle);
                if let Some(transform) = transforms.get(entity) {
                    let translation = transform.translation();
                    println!("S {} {}", translation.x, translation.y);
                    physics.set_location(&phys_handle, translation.x as f32, translation.y as f32);
                } else {
                    transforms.insert(entity, Transform::default());
                }
                handles.insert(entity, phys_handle);
            }
            for (child_entity, parent, attached) in (&entities, &parent, &mut attached).join() {
                if parent.entity == entity {
                    if let Some(handle) = handles.get(entity) {
                        if attached.handle.is_none() {
                            println!("Adding sensor!");
                            let sensor_handle =
                                physics.add_child_collider(handle, &attached.collider);
                            attached.set_handle((handle.body.unwrap(), sensor_handle));
                        }
                    }
                }
            }
        }
        for (entity, handle) in (&entities, &mut handles).join() {
            if handle.fresh {
                if let Some(body_handle) = handle.body {
                    if let Some(body) = physics.bodies.rigid_body_mut(body_handle) {
                        body.set_user_data(Some(Box::new(entity)));
                    }
                }
                if let Some(coll_handle) = handle.collider {
                    if let Some(collider) = physics.colliders.get_mut(coll_handle) {
                        collider.set_user_data(Some(Box::new(entity)));
                    }
                }
                handle.fresh = false;
            }
        }
        for (entity, attached) in (&entities, &mut attached).join() {
            if attached.fresh {
                if let Some((_, coll_handle)) = attached.handle {
                    if let Some(collider) = physics.colliders.get_mut(coll_handle) {
                        collider.set_user_data(Some(Box::new(entity)));
                    }
                }
            }
        }
    }
}

struct PhysicsDeletionSystem;

impl<'s> System<'s> for PhysicsDeletionSystem {
    type SystemData = (Write<'s, Physics>, Entities<'s>);

    fn run(&mut self, (mut physics, entities): Self::SystemData) {
        let mut bodies_to_remove = Vec::new();
        let mut colliders_to_remove = Vec::new();
        for (handle, body) in physics.bodies.iter() {
            if let Some(entity) = physics.get_body_entity(handle) {
                if !entities.is_alive(*entity) {
                    bodies_to_remove.push(handle);
                }
            }
        }
        for (handle, collider) in physics.colliders.iter() {
            if let Some(entity) = physics.get_collider_entity(handle) {
                if !entities.is_alive(*entity) {
                    colliders_to_remove.push(handle);
                }
            }
        }
        for handle in bodies_to_remove.iter() {
            physics.bodies.remove(*handle);
        }
        for handle in colliders_to_remove.iter() {
            physics.colliders.remove(*handle);
        }
    }
}

pub struct PhysicsBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for PhysicsBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(PhysicsSpawningSystem, "physics_spawn", &[]);
        dispatcher.add(PhysicsSystem, "physics", &["physics_spawn"]);
        dispatcher.add(PhysicsDeletionSystem, "physics_delete", &[]);
        // dispatcher.add(BounceSystem, "bounce", &[]);
        Ok(())
    }
}
