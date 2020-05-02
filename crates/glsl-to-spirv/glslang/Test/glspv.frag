#version 330

#ifdef GL_SPIRV
#error GL_SPIRV is set ( correct, not an error )
#if GL_SPIRV == 100
#error GL_SPIR is 100
#endif
#endif

void main()
{
}

layout(input_attachment_index = 1) uniform subpassInput sub; // ERROR, no inputs
