use crate::{
    parse_stream::{Parse, ParseError, ParseStream},
    types::{
        Bool, Colon, Comma, DoubleColon, DoubleMinus, Float, Hash, Ident, Int, LBrace, LBracket,
        LParen, RBrace, RParen, StringLit,
    },
    types::{
        Bsn, BsnEntry, BsnFields, BsnNamedField, BsnRoot, BsnScene, BsnSceneList,
        BsnSceneListItems, BsnType, BsnValue, Path, PathSegment,
    },
};

impl Parse for BsnRoot {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        Ok(BsnRoot(input.parse::<Bsn>()?))
    }
}

impl Parse for Bsn {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        let mut entries = Vec::new();
        while !(input.is_empty_or_closing_delimiter() || input.peek::<DoubleMinus>()) {
            entries.push(input.parse::<BsnEntry>()?);
        }
        Ok(Bsn { entries })
    }
}

impl Parse for BsnEntry {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        Ok(if input.peek::<Hash>() {
            let _ = input.parse::<Hash>()?;
            let ident = input.parse::<Ident>()?;
            BsnEntry::Name(ident)
        } else if input.peek::<Colon>() {
            let _ = input.parse::<Colon>();
            let path = input.parse::<StringLit>()?;
            BsnEntry::CachedScene(BsnScene::Asset(path))
        } else {
            let path = input.parse::<Path>()?;
            if input.peek::<LBracket>() {
                let scene_list = input.parse::<BsnSceneList>()?;
                BsnEntry::RelatedSceneList {
                    relationship_target_path: path,
                    scene_list,
                }
            } else {
                BsnEntry::FromTemplatePatch(BsnType {
                    path,
                    variant: None,
                    fields: input.parse::<BsnFields>()?,
                })
            }
        })
    }
}

impl Parse for BsnType {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        let path = input.parse::<Path>()?;
        let fields = input.parse::<BsnFields>()?;
        Ok(BsnType {
            path,
            variant: None,
            fields,
        })
    }
}

impl Parse for BsnFields {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        Ok(if input.peek::<LBrace>() {
            let _ = input.parse::<LBrace>()?;
            let mut fields = Vec::new();
            while !input.is_empty_or_closing_delimiter() {
                fields.push(input.parse::<BsnNamedField>()?);
                if input.is_empty_or_closing_delimiter() {
                    break;
                }
                input.parse::<Comma>()?;
            }
            let _ = input.parse::<RBrace>()?;
            BsnFields::NamedFields(fields)
        } else if input.peek::<LParen>() {
            let _ = input.parse::<LParen>()?;
            let mut fields = Vec::new();
            while !input.is_empty_or_closing_delimiter() {
                fields.push(input.parse::<BsnValue>()?);
                if input.is_empty_or_closing_delimiter() {
                    break;
                }
                input.parse::<Comma>()?;
            }
            let _ = input.parse::<RParen>()?;
            BsnFields::UnnamedFields(fields)
        } else {
            BsnFields::Unit
        })
    }
}

impl Parse for BsnNamedField {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        let name = input.parse::<Ident>()?;
        let _ = input.parse::<Colon>()?;
        let value = input.parse::<BsnValue>()?;
        Ok(BsnNamedField {
            name: name.0,
            value,
        })
    }
}

impl Parse for BsnValue {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        Ok(if input.peek::<Float>() {
            let value = input.parse::<Float>()?;
            BsnValue::Float(value.0)
        } else if input.peek::<Int>() {
            let value = input.parse::<Int>()?;
            BsnValue::Int(value.0)
        } else if input.peek::<StringLit>() {
            let value = input.parse::<StringLit>()?;
            BsnValue::String(value.0)
        } else if input.peek::<Bool>() {
            let value = input.parse::<Bool>()?;
            BsnValue::Bool(value.0)
        } else if input.peek::<Ident>() {
            let value = input.parse::<BsnType>()?;
            BsnValue::Type(value)
        } else {
            return Err(input.error("Expected a value type"));
        })
    }
}

impl Parse for BsnSceneList {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        let items = input.bracketed::<BsnSceneListItems>()?;
        Ok(BsnSceneList(items))
    }
}

impl Parse for BsnSceneListItems {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        let mut scenes = Vec::new();
        while !input.is_empty_or_closing_delimiter() {
            scenes.push(input.parse::<Bsn>()?);
            if input.is_empty_or_closing_delimiter() {
                break;
            }
            input.parse::<DoubleMinus>()?;
        }

        Ok(BsnSceneListItems(scenes))
    }
}

impl Parse for Path {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        let leading_colon = if input.peek::<DoubleColon>() {
            let _ = input.parse::<DoubleColon>()?;
            true
        } else {
            false
        };
        let mut segments = Vec::new();
        segments.push(input.parse::<PathSegment>()?);
        while input.peek::<DoubleColon>() {
            let _ = input.parse::<DoubleColon>()?;
            segments.push(input.parse::<PathSegment>()?);
        }
        Ok(Path {
            leading_colon,
            segments,
        })
    }
}

impl Parse for PathSegment {
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError<'a>> {
        let ident = input.parse::<Ident>()?;
        Ok(PathSegment { ident })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parse_stream::ParseStream,
        span::Span,
        types::{
            Bsn, BsnEntry, BsnFields, BsnNamedField, BsnRoot, BsnScene, BsnSceneList,
            BsnSceneListItems, BsnType, BsnValue, Path, StringLit,
        },
    };

    #[test]
    fn parse_bsn() {
        let mut input = ParseStream::from(Span::from(
            r#"
#A
:"inherited.bsn"
bevy_transform::Transform { x: 1.0 }
Children [
    bevy_transform::Transform
    Foo { nested: Bar { value: 1 } }
    Marker
    Score(0, 1)
    --
    #B
    Marker
    Children [
        #C
    ]
]
"#,
        ));
        let bsn_root = input.parse::<BsnRoot>().unwrap();
        assert_eq!(
            bsn_root,
            BsnRoot(Bsn {
                entries: vec![
                    BsnEntry::Name("A".into()),
                    BsnEntry::CachedScene(BsnScene::Asset(StringLit("inherited.bsn".into()))),
                    BsnEntry::FromTemplatePatch(BsnType::new(
                        ["bevy_transform", "Transform"],
                        BsnFields::NamedFields(vec![BsnNamedField::new("x", BsnValue::Float(1.0))])
                    )),
                    BsnEntry::RelatedSceneList {
                        relationship_target_path: Path::from(["Children"]),
                        scene_list: BsnSceneList(BsnSceneListItems(vec![
                            Bsn {
                                entries: vec![
                                    BsnEntry::FromTemplatePatch(BsnType::path([
                                        "bevy_transform",
                                        "Transform"
                                    ])),
                                    BsnEntry::FromTemplatePatch(BsnType::new(
                                        ["Foo"],
                                        BsnFields::NamedFields(vec![BsnNamedField::new(
                                            "nested",
                                            BsnValue::Type(BsnType::new(
                                                ["Bar"],
                                                BsnFields::NamedFields(vec![BsnNamedField::new(
                                                    "value",
                                                    BsnValue::Int(1)
                                                )])
                                            ))
                                        )])
                                    )),
                                    BsnEntry::FromTemplatePatch(BsnType::path(["Marker"])),
                                    BsnEntry::FromTemplatePatch(BsnType::new(
                                        ["Score"],
                                        BsnFields::UnnamedFields(vec![
                                            BsnValue::Int(0),
                                            BsnValue::Int(1)
                                        ])
                                    ))
                                ]
                            },
                            Bsn {
                                entries: vec![
                                    BsnEntry::Name("B".into()),
                                    BsnEntry::FromTemplatePatch(BsnType::path(["Marker"])),
                                    BsnEntry::RelatedSceneList {
                                        relationship_target_path: Path::from(["Children"]),
                                        scene_list: BsnSceneList(BsnSceneListItems(vec![Bsn {
                                            entries: vec![BsnEntry::Name("C".into())]
                                        },]))
                                    }
                                ]
                            }
                        ]))
                    }
                ]
            })
        );
    }
}
