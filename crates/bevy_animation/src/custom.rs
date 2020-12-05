struct ClipBind {
    entities: Vec<Option<Entity>>,
}

pub struct Animator {
    clips: Vec<Handle<Clip>>,
    binds: Vec<Option<ClipBind>>,
}

struct ClipProperties(HashMap<String, SmallVec<[u16; 10]>>);

pub struct AnimatorProperties {
    map: HashMap<WeakHandle<Clip>, ClipProperties>,
}

#[derive(Default)]
struct AnimatorState {
    clips_event_reader: EventReader<AssetEvent<Clip>>,
}

fn animator_udpate(
    mut state: Local<AnimatorState>,
    clips: Res<Assets<Clip>>,
    clip_events: Res<Events<AssetEvent<Clip>>>,
    mut animators_properties: ResMut<AnimatorProperties>,
    animators_query: Query<(&Animator,)>,
) {
    //
}
