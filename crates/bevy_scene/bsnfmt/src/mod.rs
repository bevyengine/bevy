#![allow(
    unused,
    reason = "Importing modules from bevy_scene_macros, but only using a few types and trait implementations"
)]
//! BSN Formatter

#[path = "../../macros/src/bsn/codegen.rs"]
mod bsn_codegen;
#[path = "../../macros/src/bsn/parse.rs"]
mod bsn_parse;
#[path = "../../macros/src/bsn/types.rs"]
mod bsn_types;

/// BSN internals
mod _bsn {
    #![allow(unused, reason = "Internal modules")]
    pub mod codegen {
        pub use crate::bsn_codegen::*;
    }
    pub mod parse {
        pub use crate::bsn_parse::*;
    }
    pub mod types {
        pub use crate::bsn_types::*;
    }
}

mod behavior;
mod traits;
pub mod types;
pub mod utils;

use crate::traits::BsnFmt;
use proc_macro2::TokenStream;
use std::{fmt::Display, str::FromStr};
use syn::parse::Parse;

/// Errors that can occur when formatting BSN
#[derive(Debug)]
pub enum FormatterError {
    /// Lexing error
    Lexing(proc_macro2::LexError),
    /// Parsing error
    Parsing(String, syn::Error),
}

impl Display for FormatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatterError::Lexing(e) => {
                write!(f, "Failed to lex input string into TokenStream: {}", e)
            }
            FormatterError::Parsing(r#type, e) => {
                write!(f, "Failed to parse input into {}: {}", r#type, e)
            }
        }
    }
}

/// TODO
pub fn format_tokens<P: BsnFmt + Parse>(
    input: TokenStream,
    base_indent: usize,
    level: usize,
) -> Result<String, FormatterError> {
    syn::parse2::<P>(input)
        .map_err(|e| FormatterError::Parsing(std::any::type_name::<P>().to_owned(), e))
        .map(|ast| ast.fmt(base_indent, level))
}

/// TODO
pub fn format_str<P: BsnFmt + Parse>(
    input: &str,
    base_indent: usize,
    level: usize,
) -> Result<String, FormatterError> {
    let tokens = TokenStream::from_str(input).map_err(FormatterError::Lexing)?;
    format_tokens::<P>(tokens, base_indent, level)
}

#[cfg(test)]
mod tests {
    use super::format_str;
    use crate::_bsn::types::BsnRoot;
    use pretty_assertions::assert_eq;

    macro_rules! test_fmt {
        ($test_name:ident, $parser:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                let input = $input;
                let expected = $expected.trim_start_matches('\n').trim_end();
                let res = format_str::<$parser>(input, 0, 0).unwrap();
                assert_eq!(
                    res, expected,
                    "\n\n BSN fmt result does not match expected. \n\ngot:\n[{}]\n\nexpected:\n[{}]\n",
                    res, expected
                );
            }
        };
    }

    test_fmt!(
        component_fields,
        BsnRoot,
        r#"
        # Name
    Component::
    Named {
        field_a: 1_000.0  ,
            field_b:     true
}
            Component::Tuple
            (
            )
    Component::Tuple(  3.0, 
       false,
    1.. 5, 0 ..
    = 10
    ,
            )
                Component:: Unit
    "#,
        r#"
    #Name
    Component::Named {
        field_a: 1_000.0,
        field_b: true,
    }
    Component::Tuple()
    Component::Tuple(3.0, false, 1..5, 0..=10)
    Component::Unit
    "#
    );

    test_fmt!(
        nested_children,
        BsnRoot,
        r#"
    #Parent Children
    [
        #ChildA --
        #ChildB
        Children[ #GrandChildA
            Children []
            -- #GrandChildB
        ]
    ]
    "#,
        r#"
    #Parent
    Children [
        #ChildA
        --
        #ChildB
        Children [
            #GrandChildA
            Children []
            --
            #GrandChildB
        ]
    ]
    "#
    );

    test_fmt!(
        basic_entity,
        BsnRoot,
        r#"
        # Name
Transform::from_xyz(10.,10.,  10.,)     .looking_at( Vec3::ZERO , Vec3::Y)
        Mesh3d(asset_value(Cuboid::from_length(1.0)))
                MeshMaterial3d <    StandardMaterial > ( asset_value(Color::srgb_u8(255, 0, 0)))
    "#,
        r#"
    #Name
    Transform::from_xyz(10., 10., 10.).looking_at(Vec3::ZERO, Vec3::Y)
    Mesh3d(asset_value(Cuboid::from_length(1.0)))
    MeshMaterial3d<StandardMaterial>(asset_value(Color::srgb_u8(255, 0, 0)))
    "#
    );

    test_fmt!(
        ui_root_node,
        BsnRoot,
        r#"{
            #Name
    Node {
                width: Val:: Percent(100.0 ) ,
                    height: Val ::Percent (100.0 ),
            align_items: AlignItems:: Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(5.),
        }
Children [
            button("Ok")
        on(|_event: On<Pointer<Press>>| println!("Ok pressed!"))
        --

            button ("Cancel")
        on(|_event: On< Pointer < Press> >| {
                    let hello =  "nested";
        println!("Cancel pressed!")
        })
        BackgroundColor(Color::srgb(0.4, 0.15, 0.15))
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
        button("Ok")
        on(|_event: On<Pointer<Press>>| println!("Ok pressed!"))
        --
        button("Cancel")
        on(|_event: On<Pointer<Press>>| {
            let hello = "nested";
            println!("Cancel pressed!")
        })
        BackgroundColor(Color::srgb(0.4, 0.15, 0.15))
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
    width: Val:: Px ( 150.0),
            height: Val ::Px(65.0),
                    border : UiRect::all(Val::Px(5.0)),
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
    Children [
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
    ]
    "#
    );

    test_fmt!(
        patching_and_inheritance,
        BsnRoot,
        r#"{
            : "scenes/player.bsn"
    @Transform {
        translation: Vec3:: new (0.0, 1.0, 0.0),
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
        nested_patches_in_children,
        BsnRoot,
        r#"{
            Node {
                    width: Val ::
                    Percent(100.0),
            }
    Children [
        (
            : "ui/button.bsn"
                    @BackgroundColor(Color::BLUE)
                    @Node {
                @margin: UiRect::all(Val::Px(10.0)),
    }
                ),
            ]
        }"#,
        r#"
    Node {
        width: Val::Percent(100.0),
    }
    Children [
        : "ui/button.bsn"
        @BackgroundColor(Color::BLUE)
        @Node {
            @margin: UiRect::all(Val::Px(10.0)),
        }
    ]
    "#
    );
}
