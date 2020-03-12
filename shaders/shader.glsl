const float INF = 1.0/0.0;
const float SHADOW_BIAS = 0.0001;

layout(constant_id = 0) const uint MAX_BOUNCES = 8;

struct Camera {
    vec3 position;
    float focusDistance;
    float aperture;
    float aspectRatio;
};

layout(push_constant) uniform PushConstants {
    Camera CAMERA;
    vec3 SKY_COLOR;
    uint SAMPLE_COUNT;
};

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

#include "geometry.glsl"
#include "rng.glsl"

struct Material {
    vec3 color;
    float mirror;
};

struct Light {
    vec3 position;
    vec3 color;
};

layout(set = 0, binding = 0, rgba32ui) uniform uimage2D result;

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
        if (hit.t > 0.0 && hit.t < bestHit.t) {
            bestHit = hit;
        }
    }

    for (uint i = 0; i < planes.length(); i++) {
        Hit hit = rayPlaneIntersect(ray, planes[i]);
        if (hit.t > 0.0 && hit.t < bestHit.t) {
            bestHit = hit;
        }
    }

    for (uint i = 0; i < triangles.length(); i++) {
        Hit hit = rayTriangleIntersect(ray, triangles[i]);
        if (hit.t > 0.0 && hit.t < bestHit.t) {
            bestHit = hit;
        }
    }

    return bestHit;
}

vec3 lightsAt(vec3 point, vec3 normal) {
    vec3 result = vec3(0.0);

    for (uint i = 0; i < lights.length(); i++) {
        Light light = lights[i];
        Ray shadowRay = Ray(point + SHADOW_BIAS * normal, normalize(light.position - point));
        Hit shadowHit = castRay(shadowRay);
        if (shadowHit.t > distance(point, light.position)) {
            result += light.color;
        }
    }

    return result;
}

struct Frame {
    vec3 lights;
    Material material;
};

vec3 trace(Ray ray) {
    Frame stack[MAX_BOUNCES];

    uint i = 0;
    vec3 result = vec3(0.0);

    //trace forward
    for (i = 0; i < MAX_BOUNCES; i++) {
        Hit hit = castRay(ray);

        if (isinf(hit.t)) {
            result = SKY_COLOR;
            break;
        } else {
            stack[i].material = materials[hit.materialIndex];
            stack[i].lights = lightsAt(hit.point, hit.normal);

            ray = Ray(hit.point, reflect(ray.direction, hit.normal));
            ray.start += SHADOW_BIAS * hit.normal;
        }
    }

    //unwinding
    while (i > 0) {
        i--;
        result = stack[i].material.color * mix(stack[i].lights, result, stack[i].material.mirror);
    }

    return result;
}

void main() {
    uvec4 seedColor = imageLoad(result, ivec2(gl_GlobalInvocationID.xy));
    uint seed = seedColor.x + (seedColor.y << 8) + (seedColor.z << 16) + (seedColor.a << 24);

    vec2 pixelPos = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(result));
    vec2 centeredPos = (vec2(pixelPos.x, 1-pixelPos.y) - vec2(0.5)) / vec2(1, CAMERA.aspectRatio);

    vec3 total = vec3(0);
    for (uint i = 0; i < SAMPLE_COUNT; i++) {
        //TODO properly use the size of a pixel here
        vec2 offset = vec2(randomFloat(seed)-0.5, randomFloat(seed)-0.5) / 1000;
        vec2 rayPos = centeredPos + offset;
        Ray primaryRay = Ray(CAMERA.position, normalize(vec3(rayPos, 1)));

        vec3 focalPoint = primaryRay.start + CAMERA.focusDistance * primaryRay.direction;
        vec3 jitterStart = primaryRay.start + CAMERA.aperture * vec3(randomUnitDisk(seed), 0.0);

        Ray secondaryRay = Ray(jitterStart, normalize(focalPoint - jitterStart));

        total += trace(secondaryRay);
    }
    total /= SAMPLE_COUNT;

    vec4 color = vec4(0, 0, 0, 1);
    color.rgb = total;

    imageStore(result, ivec2(gl_GlobalInvocationID.xy), uvec4(color*255));
}