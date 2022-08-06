fn expensive_subfunc(input: f32) -> f32 {
    return input + 1.5;
}

fn subfunc(input: f32) -> f32 {
    return input + 1.0;
}

fn test(input_one: f32, input_two: f32) -> vec2<f32> {
    var res: vec2<f32> = vec2<f32>(1.0, 1.0);

    // for(var i=0.0; i < input_two; i += 1.0) {
    //     res.x += input_one;
    //     res.y += input_two;
    // }

    // for(var i=0.0; i < input_two; i += 1.0) {
    //     res.y += input_two;
    // }

    for(var i=0.0; i < input_two; i += 1.0) {
        res.y += input_one;
    }

    res.y += expensive_subfunc(1.0);
    res.x += subfunc(3.0);
    res = res.yx;

    return res;
}