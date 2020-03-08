void xorshift(inout uint seed) {
    seed ^= seed >> 13;
    seed ^= seed << 17;
    seed ^= seed >> 5;
}

//return a random float in the range [0.0, 1.0]
float nextFloat(inout uint seed)
{
    xorshift(seed);
    return float(seed) / (pow(2.0, 32.0) - 1.0);
}