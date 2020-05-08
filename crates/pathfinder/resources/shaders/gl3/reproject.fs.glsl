#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450











precision highp float;
precision highp sampler2D;

layout(set = 0, binding = 0)uniform uOldTransform {
    mat4 oldTransformVal;
};





uniform sampler2D uTexture;


in vec2 vTexCoord;

out vec4 oFragColor;

void main(){
    vec4 normTexCoord = oldTransform * vec4(vTexCoord, 0.0, 1.0);
    vec2 texCoord =((normTexCoord . xy / normTexCoord . w)+ 1.0)* 0.5;



    oFragColor = texture(uTexture, texCoord);

}

