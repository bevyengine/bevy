struct InStruct {
    @location(0) attr_in_struct: vec4<f32>,
}

// ok
@fragment
fn fragment(
	struct_in_param: InStruct,
	@location(1) attr_in_param: vec4<f32>,
) {}


// struct NestedStruct {
//     @location(0) attr_in_nested_struct: vec4<f32>,
// }
//
// struct InStruct {
//     nested_struct: NestedStruct,
// 	   @location(1) attr_in_struct: vec4<f32>,
// }
//
// // fail
// // `Err` value: WithSpan { inner: EntryPoint { stage: Fragment, name: "fragment", error: Argument(0, MemberMissingBinding(0)) }, spans: [(Span { start: 284, end: 387 }, "naga::Type [3]")] }
// @fragment
// fn fragment(
// 	   struct_in_param: InStruct,
// ) {}