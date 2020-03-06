#version 450

const float INF = 1.0/0.0;
const float SHADOW_BIAS = 0.0001;

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D result;

struct Material {
    vec3 color;
};

struct Sphere {
    vec3 center;
    float radius;
    uint materialIndex;
};

struct Light {
    vec3 position;
    vec3 color;
};

struct Ray {
    vec3 start;
    vec3 direction;
};

//"no hit" is represent with t=INF
struct Hit {
    float t;
    vec3 point;
    vec3 normal;
    uint materialIndex;
};

layout(set = 0, binding = 1) readonly buffer Spheres {
    Sphere spheres[];
};

layout(set = 0, binding = 2) readonly buffer Lights {
    Light lights[];
};

layout(set = 0, binding = 3) readonly buffer Materials {
    Material materials[];
};

Hit raySphereIntersect(Sphere sphere, Ray ray) {
    vec3 rel = ray.start - sphere.center;

    //solve quadratic equation
    float b = 2.0 * dot(rel, ray.direction);
    float c = dot(rel, rel) - sphere.radius * sphere.radius;

    float d = b * b - 4.0 * c;
    if (d < 0.0) {
        return Hit(INF, vec3(0.0), vec3(0.0), 0);
    }

    float t_far = (-b + sqrt(d)) / 2.0;
    float t_near = (-b - sqrt(d)) / 2.0;

    float t = t_near > 0.0 ? t_near : (t_far > 0.0 ? t_far : 0.0);
    vec3 point = ray.start + t * ray.direction;
    vec3 normal = normalize(point - sphere.center);

    return Hit(t, point, normal, sphere.materialIndex);
}

Hit castRay(Ray ray) {
    Hit bestHit = Hit(INF, vec3(0.0), vec3(0.0), 0);

    for (uint i = 0; i < spheres.length(); i++) {
        Sphere sphere = spheres[i];
        Hit hit = raySphereIntersect(sphere, ray);

        if (hit.t > 0.0 && hit.t < bestHit.t) {
            bestHit = hit;
        }
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

    Ray ray = Ray(vec3(0.0), normalize(vec3(centeredPos, 1.0)));

    vec4 color = vec4(trace(ray), 1.0);
    imageStore(result, ivec2(gl_GlobalInvocationID.xy), color);
}