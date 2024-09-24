#version 430 core

in vec4 fragColor;
in vec3 fragNormal;

out vec4 finalColor;

void main()
{
    vec3 normalizedNormal = normalize(fragNormal);


    /* Excercise3 Task1 c)
    The reason for adding 1.0 is to convert the range from [-1, 1] to [0, 2], 
    and scaling it by 0.5 brings it to [0, 1], which is suitable for RGB colors.*/

    vec3 colorFromNormal = (normalizedNormal + 1.0) * 0.5;

    vec3 lightDirection = normalize(vec3(0.8, -0.5, 0.6));

    float lightIntensity = max(0.0, dot(normalizedNormal, -lightDirection));

    finalColor = vec4(lightIntensity * colorFromNormal, 1.0);
}
