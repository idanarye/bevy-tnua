use bevy_rapier3d::prelude::*;

#[derive(bevy::ecs::query::QueryData)]
pub struct PretendToBeRapierContext<'a> {
    pub simulation: &'a RapierContextSimulation,
    pub colliders: &'a RapierContextColliders,
    pub rigidbody_set: &'a RapierRigidBodySet,
}

// TODO: After https://github.com/dimforge/bevy_rapier/issues/677 is fixed, this can be reomved.
impl PretendToBeRapierContextItem<'_, '_, '_> {
    // Note that this is just a copy-paste of the function from Rapier, implemented on the Item
    // instead of the QueryData type itself (which serves more as a descriptor)
    pub fn with_query_pipeline<'a, T>(
        &'a self,
        filter: QueryFilter<'a>,
        scoped_fn: impl FnOnce(RapierQueryPipeline<'_>) -> T,
    ) -> T {
        RapierQueryPipeline::new_scoped(
            &self.simulation.broad_phase,
            self.colliders,
            self.rigidbody_set,
            &filter,
            &bevy_rapier3d::parry::query::DefaultQueryDispatcher,
            scoped_fn,
        )
    }
}
