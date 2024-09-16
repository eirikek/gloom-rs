#version 430 core

in vec3 position;
in vec4 vertexColor;

out vec4 fragColor;

uniform mat4 transformMatrix;
uniform float oscValue;

void main()
{
    mat4 modifiedMatrix = transformMatrix;

    /*
    For overview of the column-major matrix: 

    a, d, 0, 0,  First column
    b, e, 0, 0,  Second column
    0, 0, 1, 0,  Third column
    c, f, 0, 1   Fourth column

    matrixVariable[column][row] = value;

    */

    //modifiedMatrix[0][0] = oscValue;  // Modify 'a'
    //modifiedMatrix[1][0] = oscValue;  // Modify 'b'
    //modifiedMatrix[3][0] = oscValue;  // Modify 'c'
    //modifiedMatrix[0][1] = oscValue;  // Modify 'd'
    //modifiedMatrix[1][1] = oscValue;  // Modify 'e'
    //modifiedMatrix[3][1] = oscValue;  // Modify 'f'

    gl_Position = modifiedMatrix * vec4(position, 1.0f);
    fragColor = vertexColor;
}
