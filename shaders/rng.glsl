void xorshift(inout uint seed) {
    seed ^= seed >> 13;
    seed ^= seed << 17;
    seed ^= seed >> 5;
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