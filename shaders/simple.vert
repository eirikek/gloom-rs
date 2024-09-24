#version 430 core

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 vertexColor;
layout(location = 2) in vec3 normal;

out vec4 fragColor;
out vec3 fragNormal;

uniform mat4 transformMatrix;

void main()
{
    gl_Position = transformMatrix * vec4(position, 1.0);
    
    fragColor = vertexColor;
    fragNormal = normal;
}
