#version 430 core

in vec3 position;
in vec4 vertexColor;

out vec4 fragColor;

uniform mat4 transformMatrix;

void main()
{
    gl_Position = transformMatrix * vec4(position, 1.0f);
    fragColor = vertexColor;
}