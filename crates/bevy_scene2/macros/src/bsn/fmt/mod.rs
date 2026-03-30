#![allow(dead_code)]

mod behavior;
pub mod traits;
pub mod types;
mod utils;

pub use traits::*;
pub use types::*;
pub(crate) use utils::*;

#[cfg(test)]
mod fmt_tests {
    use super::*;
    use crate::bsn::types::BsnRoot;
    use pretty_assertions::assert_eq;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

    macro_rules! test_fmt {
        ($test_name:ident, $parser:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                // Arrange
                let input = $input;
                let expected = $expected.trim_start_matches('\n').trim_end();

                let tokens = TokenStream::from_str(input)
                    .expect("Failed to lex input string into TokenStream");

                let ast = syn::parse2::<$parser>(tokens)
                    .expect(concat!("Failed to parse into ", stringify!($parser)));

                // Act
                let res = ast.fmt(0, 0);

                // Assert
                assert_eq!(
                    res, expected,
                    "\n\n BSN fmt result does not match expected. \n\ngot:\n[{}]\n\nexpected:\n[{}]\n",
                    res, expected
                );
            }
        };
    }

    test_fmt!(
        ui_root_node,
        BsnRoot,
        r#"{
            #Name
    Node {
                width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(5.),
        }
        Children [
            (
                    button("Ok")
                on(|_event: On<Pointer<Press>>| println!("Ok pressed!"))
            ),
            (


                button("Cancel")
                on(|_event: On<Pointer<Press>>| {
                            let hello = "nested";
                println!("Cancel pressed!")
                })
                BackgroundColor(Color::srgb(0.4, 0.15, 0.15))
                ),
        ]
    }"#,
        r#"
    #Name
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(5.),
    }
    Children [
        (
            button("Ok")
            on(|_event: On<Pointer<Press>>| println!("Ok pressed!"))
        ),
        (
            button("Cancel")
            on(|_event: On<Pointer<Press>>| {
                let hello = "nested";
                println!("Cancel pressed!")
            })
            BackgroundColor(Color::srgb(0.4, 0.15, 0.15))
        ),
    ]
"#
    );

    test_fmt!(
        button_with_complex_template,
        BsnRoot,
        r#"
        {
            #Button
        Button
        Node {
        width: Val::Px(150.0),
                height: Val::Px(65.0),
                     border: UiRect::all(Val::Px(5.0)),
                        border_radius: BorderRadius::MAX,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
            Children [(
            Text(label)
        template(|context| {
            Ok(TextFont {
                font: context
                        .resource::<AssetServer>()
                    .load("fonts/FiraSans-Bold.ttf").into(),
        font_size: FontSize::Px(33.0),
        ..default()
            })
        })
                TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        )]
    }
    "#,
        r#"
    #Button
    Button
    Node {
        width: Val::Px(150.0),
        height: Val::Px(65.0),
        border: UiRect::all(Val::Px(5.0)),
        border_radius: BorderRadius::MAX,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
    }
    BorderColor::from(Color::BLACK)
    BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
    Children [(
        Text(label)
        template(|context| {
            Ok(TextFont {
                font: context
                    .resource::<AssetServer>()
                    .load("fonts/FiraSans-Bold.ttf")
                    .into(),
                font_size: FontSize::Px(33.0),
                ..default()
            })
        })
        TextColor(Color::srgb(0.9, 0.9, 0.9))
        TextShadow
    )]
"#
    );

    test_fmt!(
        patching_and_inheritance,
        BsnRoot,
        r#"{
            : "scenes/player.bsn"
            @Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                    rotation: Quat::IDENTITY,
            }
            @Sprite::default()
                @Visibility::Visible
                    #my_entity
        }"#,
        r#"
    : "scenes/player.bsn"
    @Transform {
        translation: Vec3::new(0.0, 1.0, 0.0),
        rotation: Quat::IDENTITY,
    }
    @Sprite::default()
    @Visibility::Visible
    #my_entity
"#
    );

    test_fmt!(
        template_constants_and_constructors,
        BsnRoot,
        r#"{
@MyComponent::CONST_VALUE
    @MyComponent::new(1, 2, 3)
        FromTemplateConstructor::create()
        }"#,
        r#"
    @MyComponent::CONST_VALUE
    @MyComponent::new(1, 2, 3)
    FromTemplateConstructor::create()
"#
    );

    test_fmt!(
        nested_patches_in_children,
        BsnRoot,
        r#"{
            Node {
                    width: Val::Percent(100.0),
            }
            Children [
                (
                    : "ui/button.bsn"
                            @BackgroundColor(Color::BLUE)
                            @Node {
                        margin: UiRect::all(Val::Px(10.0)),
            }
                ),
            ]
        }"#,
        r#"
    Node {
        width: Val::Percent(100.0),
    }
    Children [(
        : "ui/button.bsn"
        @BackgroundColor(Color::BLUE)
        @Node {
            margin: UiRect::all(Val::Px(10.0)),
        }
    )]
"#
    );
}
