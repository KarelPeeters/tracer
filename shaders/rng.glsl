void xorshift(inout uint seed) {
    seed ^= seed >> 13;
    seed ^= seed << 17;
    seed ^= seed >> 5;
}

bool randomBool(inout uint seed) {
    xorshift(seed);
    return bool(seed & uint(1));
}

//return a random float in the range [0.0, 1.0]
float randomFloat(inout uint seed)
{
    xorshift(seed);
    return float(seed) / (pow(2.0, 32.0) - 1.0);
}

//return a random point in the unit disk with a uniform distribution
vec2 randomUnitDisk(inout uint seed) {
    float r = sqrt(randomFloat(seed));
    float t = randomFloat(seed) * 2 * 3.1415926;

    return vec2(cos(t), sin(t)) * r;
}

//returns a random vector on the unit hemisphere towards normal, weighed by the consine of the angle between them
vec3 randomCosineUnitHemi(inout uint seed, vec3 normal) {
    vec2 disk = randomUnitDisk(seed);
    float z = sqrt(1.0 - dot(disk, disk));

    vec3 xaxis = normalize(vec3(-normal.y, normal.x, 0.0));
    vec3 yaxis = cross(normal, xaxis);

    return disk.x * xaxis + disk.y * yaxis + z * normal;
}