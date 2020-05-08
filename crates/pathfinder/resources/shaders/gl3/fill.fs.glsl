#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450











precision highp float;
precision highp sampler2D;





uniform sampler2D uAreaLUT;


in vec2 vFrom;
in vec2 vTo;

out vec4 oFragColor;

void main(){

    vec2 from = vFrom, to = vTo;


    vec2 left = from . x < to . x ? from : to, right = from . x < to . x ? to : from;


    vec2 window = clamp(vec2(from . x, to . x), - 0.5, 0.5);
    float offset = mix(window . x, window . y, 0.5)- left . x;
    float t = offset /(right . x - left . x);


    float y = mix(left . y, right . y, t);
    float d =(right . y - left . y)/(right . x - left . x);


    float dX = window . x - window . y;
    vec2 lutTexCoord = vec2(y + 8.0, abs(d * dX))/ 16.0;



    vec4 lutColor = texture(uAreaLUT, lutTexCoord);

    oFragColor = vec4(lutColor . r * dX);
}

