const float INF = 1.0/0.0;
const float SHADOW_BIAS = 0.0001;

layout(constant_id = 0) const uint MAX_BOUNCES = 8;

struct Camera {
    vec3 position;
    vec3 direction;
    float focusDistance;
    float aperture;
    float aspectRatio;

    vec3 startVolumetricMask;
    float startScatteringCoef;
};

layout(push_constant) uniform PushConstants {
    Camera CAMERA;
    vec3 SKY_COLOR;
    uint SAMPLE_COUNT;
    bool SAMPLE_LIGHTS;
};

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

#include "geometry.glsl"
#include "rng.glsl"

// 0.0            .. keyDiffuse: diffuse
// keyDiffuse     .. keyTransparent: mirror
// keyTransparent .. 1.0: transparent
struct Material {
    vec3 color;
    bool fixedColor;

    vec3 volumetricColor;
    float scatteringCoef;

    float refractRatio;//when going against normal

    float keyDiffuse;
    float keyTransparent;
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
            result += light.color * abs(dot(normal, shadowRay.direction));
        }
    }

    return result;
}

struct Frame {
    vec3 lights;
    Material material;
};

vec3 pow(vec3 b, float e) {
    return vec3(pow(b.x, e), pow(b.y, e), pow(b.z, e));
}

vec3 powInf(vec3 b) {
    return mix(vec3(0.0), vec3(1.0), lessThan(abs(b-1.0), vec3(0.00001)));
}

vec3 divKeepZero(vec3 a, vec3 b) {
    a /= b;
    return mix(a, vec3(0.0), isnan(a));
}

vec3 trace(Ray ray, inout uint seed) {
    vec3 result = vec3(0.0);
    vec3 mask = vec3(1.0);

    vec3 volumetricMask = CAMERA.startVolumetricMask;
    float scatteringCoef = CAMERA.startScatteringCoef;

    for (uint i = 0; i < MAX_BOUNCES; i++) {
        Hit hit = castRay(ray);

        if (isinf(hit.t)) {
            mask *= powInf(volumetricMask);
            result += mask * SKY_COLOR;
            break;
        } else {
            float scatterT = -log(randomFloat(seed))/scatteringCoef;
            if (scatterT < hit.t) {
                //scatter
                mask *= pow(volumetricMask, scatterT);
                vec3 position = ray.start + scatterT * ray.direction;
                vec3 nextDir = ray.direction;//randomUnitSphere(seed);
                ray = Ray(position, nextDir);
                continue;
            }

            //continue to next hit
            Material material = materials[hit.materialIndex];
            mask *= material.color;
            mask *= pow(volumetricMask, hit.t);

            if (material.fixedColor) {
                return mask;
            }

            vec3 nextDir;
            vec3 nextStart;

            float key = randomFloat(seed);

            if (key > material.keyTransparent) {
                //transparent
                float r = material.refractRatio;
                float c = - dot(hit.normal, ray.direction);
                vec3 normal = hit.normal;

                bool into = c > 0.0;
                bool outOf = c < 0.0;
                if (!into) {
                    r = 1.0/r;
                    c = -c;
                    normal = -normal;
                }

                float x = 1.0 - r*r*(1-c*c);
                if (x >= 0.0) {
                    //actual transparancy
                    if (into) {
                        volumetricMask *= material.volumetricColor;
                        scatteringCoef += material.scatteringCoef;
                    }
                    if (outOf) {
                        //if volumetricColor is zero is means the mask is going to be zero for that color anyway
                        volumetricMask = divKeepZero(volumetricMask, material.volumetricColor);
                        scatteringCoef -= material.scatteringCoef;
                    }

                    nextDir = r * ray.direction + (r * c - sqrt(x)) * normal;
                    nextStart = hit.point + SHADOW_BIAS * nextDir;
                } else {
                    //total internal reflection
                    nextDir = reflect(ray.direction, hit.normal);
                    nextStart = hit.point - SHADOW_BIAS * hit.normal;
                }
            } else {
                //non transparent

                //diffuse lights
                if (SAMPLE_LIGHTS) {
                    vec3 lights = lightsAt(hit.point, hit.normal);
                    result += mask * material.keyDiffuse * lights;
                }

                if (key < material.keyDiffuse) {
                    //diffuse
                    nextDir = randomCosineUnitHemi(seed, hit.normal);
                } else {
                    //mirror
                    nextDir = reflect(ray.direction, hit.normal);
                }

                nextStart = hit.point + SHADOW_BIAS * hit.normal;
            }

            ray = Ray(nextStart, nextDir);
        }
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
        vec2 offset = vec2(randomFloat(seed)-0.5, randomFloat(seed)-0.5) / imageSize(result);
        vec2 rayPos = centeredPos + offset;
        Ray primaryRay = Ray(CAMERA.position, normalize(vec3(rayPos, 0.0) + CAMERA.direction));

        vec3 focalPoint = primaryRay.start + CAMERA.focusDistance * primaryRay.direction;
        vec3 jitterStart = primaryRay.start + CAMERA.aperture * vec3(randomUnitDisk(seed), 0.0);

        Ray secondaryRay = Ray(jitterStart, normalize(focalPoint - jitterStart));

        total += trace(secondaryRay, seed);
    }
    total /= SAMPLE_COUNT;

    vec4 color = vec4(0, 0, 0, 1);
    color.rgb = total;

    imageStore(result, ivec2(gl_GlobalInvocationID.xy), uvec4(color*255));
}