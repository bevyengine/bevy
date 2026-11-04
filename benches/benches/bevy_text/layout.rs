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
    "Call me Ishmael. Some years ago—never mind how long precisely—having little or no money in my purse, and nothing particular to interest me on shore, I thought I would sail about a little and see the watery part of the world. It is a way I have of driving off the spleen and regulating the circulation. Whenever I find myself growing grim about the mouth; whenever it is a damp, drizzly November in my soul; whenever I find myself involuntarily pausing before coffin warehouses, and bringing up the rear of every funeral I meet; and especially whenever my hypos get such an upper hand of me, that it requires a strong moral principle to prevent me from deliberately stepping into the street, and methodically knocking people’s hats off—then, I account it high time to get to sea as soon as I can. This is my substitute for pistol and ball. With a philosophical flourish Cato throws himself upon his sword; I quietly take to the ship. There is nothing surprising in this. If they but knew it, almost all men in their degree, some time or other, cherish very nearly the same feelings towards the ocean with me.\n",
    "There now is your insular city of the Manhattoes, belted round by wharves as Indian isles by coral reefs—commerce surrounds it with her surf. Right and left, the streets take you waterward. Its extreme downtown is the battery, where that noble mole is washed by waves, and cooled by breezes, which a few hours previous were out of sight of land. Look at the crowds of water-gazers there.\n",
    "Circumambulate the city of a dreamy Sabbath afternoon. Go from Corlears Hook to Coenties Slip, and from thence, by Whitehall, northward. What do you see?—Posted like silent sentinels all around the town, stand thousands upon thousands of mortal men fixed in ocean reveries. Some leaning against the spiles; some seated upon the pier-heads; some looking over the bulwarks of ships from China; some high aloft in the rigging, as if striving to get a still better seaward peep. But these are all landsmen; of week days pent up in lath and plaster—tied to counters, nailed to benches, clinched to desks. How then is this? Are the green fields gone? What do they here?\n",
    "But look! here come more crowds, pacing straight for the water, and seemingly bound for a dive. Strange! Nothing will content them but the extremest limit of the land; loitering under the shady lee of yonder warehouses will not suffice. No. They must get just as nigh the water as they possibly can without falling in. And there they stand—miles of them—leagues. Inlanders all, they come from lanes and alleys, streets and avenues—north, east, south, and west. Yet here they all unite. Tell me, does the magnetic virtue of the needles of the compasses of all those ships attract them thither?\n",
    "Once more. Say you are in the country; in some high land of lakes. Take almost any path you please, and ten to one it carries you down in a dale, and leaves you there by a pool in the stream. There is magic in it. Let the most absent-minded of men be plunged in his deepest reveries—stand that man on his legs, set his feet a-going, and he will infallibly lead you to water, if water there be in all that region. Should you ever be athirst in the great American desert, try this experiment, if your caravan happen to be supplied with a metaphysical professor. Yes, as every one knows, meditation and water are wedded for ever."
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
