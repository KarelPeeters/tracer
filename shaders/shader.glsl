const float INF = 1.0/0.0;
const float SHADOW_BIAS = 0.0001;

const vec3 CAMERA_POS = vec3(0.0, 1.0, 0.0);

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

#include "geometry.glsl"

struct Material {
    vec3 color;
};

struct Light {
    vec3 position;
    vec3 color;
};

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D result;

layout(set = 0, binding = 1) readonly buffer Materials {
    Material materials[];
};

layout(set = 0, binding = 2) readonly buffer Lights {
    Light lights[];
};

layout(set = 0, binding = 3) readonly buffer Spheres {
    Sphere spheres[];
};

layout(set = 0, binding = 4) readonly buffer Planes {
    Plane planes[];
};

layout(set = 0, binding = 5) readonly buffer Triangles {
    Triangle triangles[];
};

Hit castRay(Ray ray) {
    Hit bestHit = Hit(INF, vec3(0.0), vec3(0.0), 0);

    for (uint i = 0; i < spheres.length(); i++) {
        Hit hit = raySphereIntersect(ray, spheres[i]);
        if (hit.t > 0.0 && hit.t < bestHit.t)
            bestHit = hit;
    }

    for (uint i = 0; i < planes.length(); i++) {
        Hit hit = rayPlaneIntersect(ray, planes[i]);
        if (hit.t > 0.0 && hit.t < bestHit.t)
            bestHit = hit;
    }

    for (uint i = 0; i < triangles.length(); i++) {
        Hit hit = rayTriangleIntersect(ray, triangles[i]);
        if (hit.t > 0.0 && hit.t < bestHit.t)
            bestHit = hit;
    }

    return bestHit;
}

vec3 trace(Ray ray) {
    vec3 color = vec3(0.0, 0.0, 0.0);

    Hit hit = castRay(ray);
    Material material = materials[hit.materialIndex];

    if (!isinf(hit.t)) {
        for (uint i = 0; i < lights.length(); i++) {
            Light light = lights[i];
            Ray shadowRay = Ray(hit.point + SHADOW_BIAS * hit.normal, normalize(light.position - hit.point));
            Hit shadowHit = castRay(shadowRay);
            if (shadowHit.t > distance(hit.point, light.position)) {
                //light is visible
                color.rgb += material.color * light.color;
            }
        }
    }

    return color;
}

void main() {
    vec2 pixelPos = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(result));
    vec2 centeredPos = vec2(pixelPos.x, 1.0-pixelPos.y) - vec2(0.5);

    Ray ray = Ray(CAMERA_POS, normalize(vec3(centeredPos, 1.0)));

    vec4 color = vec4(trace(ray), 1.0);
    imageStore(result, ivec2(gl_GlobalInvocationID.xy), color);
}