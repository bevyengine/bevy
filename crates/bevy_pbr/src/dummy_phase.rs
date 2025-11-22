use core::{marker::PhantomData, ops::Range};

use bevy_core_pipeline::core_3d::{Opaque3dBatchSetKey, Opaque3dBinKey};
use bevy_ecs::entity::Entity;
use bevy_render::{
    render_phase::{
        BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem,
        PhaseItemExtraIndex,
    },
    render_resource::CachedRenderPipelineId,
    sync_world::MainEntity,
};

use crate::{
    BinnedPhaseFamily, MeshPass, NoExtractCondition, PIEPhase, PhaseContext, PhaseItemExt,
    PhaseItems, RenderPhaseType,
};

const DUMMY_PHASE_ERROR: &str = "Dummy phase should never be instantiated.";

macro_rules! define_dummy_phase {
    ($name:ident) => {
        pub struct $name<P>(PhantomData<P>);

        impl<P: MeshPass> PhaseItemExt for $name<P> {
            // Important: It must be empty to ensure it does not match any material.
            const PHASE_TYPES: RenderPhaseType = RenderPhaseType::empty();

            type PhaseFamily = BinnedPhaseFamily<Self>;
            type ExtractCondition = NoExtractCondition;

            fn queue(_render_phase: &mut PIEPhase<Self>, _context: &PhaseContext) {
                panic!("{}", DUMMY_PHASE_ERROR)
            }
        }

        impl<P: MeshPass> PhaseItem for $name<P> {
            fn entity(&self) -> Entity {
                panic!("{}", DUMMY_PHASE_ERROR)
            }

            fn main_entity(&self) -> MainEntity {
                panic!("{}", DUMMY_PHASE_ERROR)
            }

            fn draw_function(&self) -> DrawFunctionId {
                panic!("{}", DUMMY_PHASE_ERROR)
            }

            fn batch_range(&self) -> &Range<u32> {
                panic!("{}", DUMMY_PHASE_ERROR)
            }

            fn batch_range_mut(&mut self) -> &mut Range<u32> {
                panic!("{}", DUMMY_PHASE_ERROR)
            }

            fn extra_index(&self) -> PhaseItemExtraIndex {
                panic!("{}", DUMMY_PHASE_ERROR)
            }

            fn batch_range_and_extra_index_mut(
                &mut self,
            ) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
                panic!("{}", DUMMY_PHASE_ERROR)
            }
        }

        impl<P: MeshPass> BinnedPhaseItem for $name<P> {
            type BatchSetKey = Opaque3dBatchSetKey;
            type BinKey = Opaque3dBinKey;

            fn new(
                _batch_set_key: Self::BatchSetKey,
                _bin_key: Self::BinKey,
                _representative_entity: (Entity, MainEntity),
                _batch_range: Range<u32>,
                _extra_index: PhaseItemExtraIndex,
            ) -> Self {
                panic!("{}", DUMMY_PHASE_ERROR)
            }
        }

        impl<P: MeshPass> CachedRenderPipelinePhaseItem for $name<P> {
            fn cached_pipeline(&self) -> CachedRenderPipelineId {
                panic!("{}", DUMMY_PHASE_ERROR)
            }
        }
    };
}

define_dummy_phase!(DummyPhase2);
define_dummy_phase!(DummyPhase3);
define_dummy_phase!(DummyPhase4);

impl<P, PIE> PhaseItems<P> for PIE
where
    P: MeshPass,
    PIE: PhaseItemExt,
{
    type PIE1 = PIE;
    type PIE2 = DummyPhase2<P>;
    type PIE3 = DummyPhase3<P>;
    type PIE4 = DummyPhase4<P>;

    fn count() -> usize {
        1
    }
}

impl<P, PIE1> PhaseItems<P> for (PIE1,)
where
    P: MeshPass,
    PIE1: PhaseItemExt,
{
    type PIE1 = PIE1;
    type PIE2 = DummyPhase2<P>;
    type PIE3 = DummyPhase3<P>;
    type PIE4 = DummyPhase4<P>;

    fn count() -> usize {
        1
    }
}

impl<P, PIE1, PIE2> PhaseItems<P> for (PIE1, PIE2)
where
    P: MeshPass,
    PIE1: PhaseItemExt,
    PIE2: PhaseItemExt,
{
    type PIE1 = PIE1;
    type PIE2 = PIE2;
    type PIE3 = DummyPhase3<P>;
    type PIE4 = DummyPhase4<P>;

    fn count() -> usize {
        2
    }
}

impl<P, PIE1, PIE2, PIE3> PhaseItems<P> for (PIE1, PIE2, PIE3)
where
    P: MeshPass,
    PIE1: PhaseItemExt,
    PIE2: PhaseItemExt,
    PIE3: PhaseItemExt,
{
    type PIE1 = PIE1;
    type PIE2 = PIE2;
    type PIE3 = PIE3;
    type PIE4 = DummyPhase4<P>;

    fn count() -> usize {
        3
    }
}

impl<P, PIE1, PIE2, PIE3, PIE4> PhaseItems<P> for (PIE1, PIE2, PIE3, PIE4)
where
    P: MeshPass,
    PIE1: PhaseItemExt,
    PIE2: PhaseItemExt,
    PIE3: PhaseItemExt,
    PIE4: PhaseItemExt,
{
    type PIE1 = PIE1;
    type PIE2 = PIE2;
    type PIE3 = PIE3;
    type PIE4 = PIE4;

    fn count() -> usize {
        4
    }
}
