use criterion::criterion_group;
use criterion::Criterion;

criterion_group!(benches, layout);

const texts: [&str; 5] = [
    "Lorem ipsum dolor sit amet consectetur adipiscing elit veniam. Dolorem lorem irure officia \
    sint irure adipiscing. Consequat voluptas exercitation assumenda eiusmod accusamus. Et quis ex \
    autem magna mollitia voluptate. Soluta culpa amet cupidatat maxime ea elit sint vel nihil. ",
    "Ullamco est cupiditate consequatur iusto voluptas nulla deleniti. Mollit veniam dolor nulla et \
    dolor sunt laborum in blanditiis nobis fuga. Fugiat exercitation occaecat sint est fugiat sint \
    temporibus quas et. Est tempore aliquip amet temporibus imperdiet ullamco consequatur sed \
    adipiscing. ",
    "Nobis dolorem sed animi repellendus ut. Dolorem quo aute temporibus qui ex dolorum cupiditate \
    quos voluptas. Eiusmod occaecat voluptas nam velit culpa culpa occaecat occaecat. Tempor \
    cillum anim pariatur temporibus distinctio ut voluptate accusamus in quas minus praesentium. ",
    "Id est lorem deleniti dignissimos quo incididunt. Esse harum ut quos assumenda fugiat \
    excepturi exercitation culpa aliqua incididunt animi sint. Laborum voluptate cupiditate non ex \
    est blanditiis. ",
    "Quod quos eos accusamus et eum blanditiis occaecat. Maxime molestias nam anim est qui aliquip \
    quas et temporibus irure voluptas assumenda. Harum repellendus sint aliquip nihil possimus \
    accusamus. Excepteur voluptas provident veniam dolores est dolore veniam."
];

#[derive(Component)]
struct BenchText(String);

fn setup_app() -> bevy_app::App {
    use bevy_app::{App, Update};
    use bevy_asset::{AssetId, Assets};
    use bevy_ecs::schedule::IntoScheduleConfigs;
    use bevy_text::{DefaultFontSource, Font, FontCx, LayoutCx, RemSize, TextPipeline};
    use bevy_ui::{ui_surface::UiSurface, widget::Text, Node, Val};

    let mut app = App::new();
    app.init_resource::<Assets<Font>>()
        .init_resource::<FontCx>()
        .init_resource::<LayoutCx>()
        .init_resource::<TextPipeline>()
        .init_resource::<RemSize>()
        .init_resource::<DefaultFontSource>()
        .add_systems(
            Update,
            (
                bevy_text::load_font_assets_into_font_collection,
                bevy_text::detect_text_needs_rerender,
            )
                .chain(),
        );

    app.world_mut()
        .resource_mut::<Assets<Font>>()
        .insert(
            AssetId::default(),
            Font::from_bytes(
                include_bytes!("../../../crates/bevy_text/src/FiraMono-subset.ttf").to_vec(),
            ),
        )
        .unwrap();
    app
}

fn layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_layout");
}
