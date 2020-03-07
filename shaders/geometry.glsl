struct Sphere {
    vec3 center;
    float radius;
    uint materialIndex;
};

struct Plane {
    float dist;
    vec3 normal;
    uint materialIndex;
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

Hit raySphereIntersect(Ray ray, Sphere sphere) {
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

    float t = t_near > 0.0 ? t_near : (t_far > 0.0 ? t_far : INF);
    vec3 point = ray.start + t * ray.direction;
    vec3 normal = normalize(point - sphere.center);

    return Hit(t, point, normal, sphere.materialIndex);
}

Hit rayPlaneIntersect(Ray ray, Plane plane) {
    float num = plane.dist + dot(ray.start, plane.normal);
    float den = dot(ray.direction, plane.normal);
    float t = -num / den;

    if (t < 0.0 || isinf(t)) {
        return Hit(INF, vec3(0.0), vec3(0.0), 0);
    }

    vec3 point = ray.start + t * ray.direction;
    return Hit(t, point, plane.normal, plane.materialIndex);
}