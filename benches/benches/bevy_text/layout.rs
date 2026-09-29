use bevy_app::{App, Update};
use bevy_asset::{AssetId, Assets};
use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use bevy_text::TextSpan;
use bevy_text::{
    ComputedTextBlock, DefaultFontSource, Font, FontCx, LayoutCx, LetterSpacing, LineHeight,
    RemSize, TextBounds, TextColor, TextFont, TextLayout, TextPipeline, TextReader, TextSection,
};
use criterion::criterion_group;
use criterion::Criterion;

criterion_group!(benches, layout);

const TEXTS: [&str; 5] = [
    "Lorem ipsum dolor sit amet consectetur adipiscing elit veniam. Dolorem lorem irure officia \
    sint irure adipiscing. Consequat voluptas exercitation assumenda eiusmod accusamus. Et quis ex \
    autem magna mollitia voluptate. Soluta culpa amet cupidatat maxime ea elit sint vel nihil.\n",
    "Ullamco est cupiditate consequatur iusto voluptas nulla deleniti. Mollit veniam dolor nulla et \
    dolor sunt laborum in blanditiis nobis fuga. Fugiat exercitation occaecat sint est fugiat sint \
    temporibus quas et. Est tempore aliquip amet temporibus imperdiet ullamco consequatur sed \
    adipiscing.\n",
    "Nobis dolorem sed animi repellendus ut. Dolorem quo aute temporibus qui ex dolorum cupiditate \
    quos voluptas. Eiusmod occaecat voluptas nam velit culpa culpa occaecat occaecat. Tempor \
    cillum anim pariatur temporibus distinctio ut voluptate accusamus in quas minus praesentium.\n",
    "Id est lorem deleniti dignissimos quo incididunt. Esse harum ut quos assumenda fugiat \
    excepturi exercitation culpa aliqua incididunt animi sint. Laborum voluptate cupiditate non ex \
    est blanditiis.\n",
    "Quod quos eos accusamus et eum blanditiis occaecat. Maxime molestias nam anim est qui aliquip \
    quas et temporibus irure voluptas assumenda. Harum repellendus sint aliquip nihil possimus \
    accusamus. Excepteur voluptas provident veniam dolores est dolore veniam."
];

#[derive(Component)]
#[require(TextFont, TextLayout, TextBounds, TextColor, LineHeight, LetterSpacing)]
struct BenchText(String);

impl From<String> for BenchText {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl TextSection for BenchText {
    fn get_text(&self) -> &str {
        &self.0
    }

    fn get_text_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

fn update_text_buffers(
    fonts: Res<Assets<Font>>,
    mut font_cx: ResMut<FontCx>,
    mut layout_cx: ResMut<LayoutCx>,
    mut pipeline: ResMut<TextPipeline>,
    rem_size: Res<RemSize>,
    default_font: Res<DefaultFontSource>,
    mut reader: TextReader<BenchText>,
    mut query: Query<(Entity, &TextLayout, &TextBounds, &mut ComputedTextBlock), With<BenchText>>,
) {
    for (entity, layout, bounds, mut computed) in &mut query {
        pipeline
            .update_buffer(
                &fonts,
                reader.iter(entity),
                layout.linebreak,
                layout.justify,
                *bounds,
                1.0,
                &mut computed,
                &mut font_cx,
                &mut layout_cx,
                Vec2::splat(1000.),
                *rem_size,
                &default_font.0,
            )
            .unwrap();
    }
}

fn setup_app() -> App {
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
                update_text_buffers,
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

    group.bench_function("single_text_entity_unwrapped", |b| {
        let mut app = setup_app();
        app.world_mut().spawn(BenchText(TEXTS.concat()));
        app.update();
        b.iter(|| app.update());
    });

    group.bench_function("text_sections_unwrapped", |b| {
        let mut app = setup_app();
        app.world_mut()
            .spawn(BenchText(TEXTS[0].to_string()))
            .with_children(|builder| {
                for text in &TEXTS[1..] {
                    builder.spawn(TextSpan(text.to_string()));
                }
            });
        app.update();
        b.iter(|| app.update());
    });

    group.bench_function("single_text_entity_wrapped", |b| {
        let mut app = setup_app();
        app.world_mut()
            .spawn((BenchText(TEXTS.concat()), TextBounds::new_horizontal(300.)));
        app.update();
        b.iter(|| app.update());
    });

    group.bench_function("text_sections_wrapped", |b| {
        let mut app = setup_app();
        app.world_mut()
            .spawn((
                BenchText(TEXTS[0].to_string()),
                TextBounds::new_horizontal(300.),
            ))
            .with_children(|builder| {
                for text in &TEXTS[1..] {
                    builder.spawn(TextSpan(text.to_string()));
                }
            });
        app.update();
        b.iter(|| app.update());
    });
}
